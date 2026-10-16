//! 10 段均衡器：RBJ peaking biquad 级联。
//!
//! 频段固定为 31Hz ~ 16kHz（每倍频程一段），增益 -12 ~ +12 dB。
//! 为防止正增益削波，前置一个预算增益（负的最大频段增益）。
//! 系数通过 `Arc<Mutex<EqState>>` 共享给音频线程：更新走锁，处理路径
//! `try_lock` 失败时沿用旧系数继续播（绝不阻塞音频回调）。

use std::sync::{Arc, Mutex, MutexGuard};

/// 均衡器频段中心频率（Hz），从低到高排列
pub const EQ_FREQS: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// peaking 滤波器品质因数
const EQ_Q: f32 = 1.0;

/// 单个 biquad 段（DF-I 实现）
#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    // 状态寄存器
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    /// 按 RBJ cookbook 计算 peaking 滤波器系数（不动状态寄存器）
    fn design(f0: f32, gain_db: f32, sample_rate: f32, q: f32) -> Biquad {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = std::f32::consts::TAU * f0 / sample_rate;
        // 频率超出奈奎斯特时（如 16kHz @ 32kHz 采样率），cos/sin 仍有限，
        // 但滤波器无意义，退化为直通
        if w0 >= std::f32::consts::PI || w0 <= 0.0 {
            return Biquad { b0: 1.0, ..Biquad::default() };
        }
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();
        let a0 = 1.0 + alpha / a;
        Biquad {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos_w0) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos_w0) / a0,
            a2: (1.0 - alpha / a) / a0,
            ..Biquad::default()
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        // 冲刷非正规数，避免音频路径性能塌陷
        self.y1 = if y.abs() < 1e-20 { 0.0 } else { y };
        y
    }
}

/// 均衡器共享状态：系数 + 各声道滤波链
pub struct EqState {
    enabled: bool,
    /// 10 段增益（dB），-12 ~ +12
    gains: [f32; 10],
    sample_rate: f32,
    channels: usize,
    /// 预算增益（防削波）
    pre_gain: f32,
    /// 每声道一条 10 段级联链
    filters: Vec<[Biquad; 10]>,
}

impl EqState {
    pub fn new() -> Self {
        EqState {
            enabled: false,
            gains: [0.0; 10],
            sample_rate: 44_100.0,
            channels: 2,
            pre_gain: 1.0,
            filters: vec![[Biquad::default(); 10]; 2],
        }
    }

    /// 重建系数（采样率/声道数/增益任一变化后调用）
    fn redesign(&mut self) {
        self.pre_gain = 10.0f32.powf(-self.gains.iter().cloned().fold(0.0, f32::max) / 20.0);
        self.filters = (0..self.channels)
            .map(|_| {
                let mut chain = [Biquad::default(); 10];
                for (i, band) in chain.iter_mut().enumerate() {
                    *band = Biquad::design(EQ_FREQS[i], self.gains[i], self.sample_rate, EQ_Q);
                }
                chain
            })
            .collect();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_gains(&mut self, gains: [f32; 10]) {
        self.gains = gains;
        self.redesign();
    }

    /// 解码会话启动时同步流参数（变更后清空滤波状态）
    pub fn set_stream_format(&mut self, sample_rate: u32, channels: usize) {
        let sr = sample_rate as f32;
        if (self.sample_rate - sr).abs() > f32::EPSILON || self.channels != channels {
            self.sample_rate = sr;
            self.channels = channels;
            self.redesign();
        }
    }

    /// 处理交错 f32 样本（就地对整段缓冲应用 EQ）
    pub fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }
        let channels = self.channels.max(1);
        let pre_gain = self.pre_gain;
        for chunk in samples.chunks_mut(channels) {
            for (ch, sample) in chunk.iter_mut().enumerate() {
                let mut value = *sample * pre_gain;
                if let Some(chain) = self.filters.get_mut(ch) {
                    for band in chain.iter_mut() {
                        value = band.process(value);
                    }
                }
                *sample = value;
            }
        }
    }
}

/// 线程安全的均衡器句柄（音频线程与命令线程共享）
#[derive(Clone)]
pub struct EqChain {
    state: Arc<Mutex<EqState>>,
}

impl EqChain {
    pub fn new() -> Self {
        EqChain { state: Arc::new(Mutex::new(EqState::new())) }
    }

    /// 阻塞更新（命令线程调用）
    pub fn update<F: FnOnce(&mut EqState)>(&self, f: F) {
        if let Ok(mut state) = self.state.lock() {
            f(&mut state);
        }
    }

    /// 音频线程调用：try_lock 失败时跳过本次处理（沿用旧系数，不阻塞）
    pub fn process(&self, samples: &mut [f32]) {
        if let Ok(mut state) = self.state.try_lock() {
            state.process(samples);
        }
    }

    #[allow(dead_code)]
    pub fn lock(&self) -> Option<MutexGuard<'_, EqState>> {
        self.state.lock().ok()
    }
}
