//! 音频输出封装：rodio 输出流与 Sink 管理。
//!
//! OutputStream 全局唯一并随引擎存活；每次加载/seek 创建新的 Sink
//! （丢弃旧 Sink 前必须显式 `stop()`，否则混音器里会残留旧音源继续出声）。

use rodio::{OutputStream, OutputStreamHandle, Sink, Source};

/// 从 ring buffer 读取样本的 rodio Source。
///
/// ring 为空时输出静音（混音器需要它持续拉取以保持设备活跃）；
/// 采样率/声道数声明为解码流的原始值，rodio 混音器会自动转换到设备参数。
pub struct RingSource {
    consumer: rtrb::Consumer<f32>,
    channels: u16,
    sample_rate: u32,
}

impl RingSource {
    pub fn new(consumer: rtrb::Consumer<f32>, channels: u16, sample_rate: u32) -> Self {
        RingSource { consumer, channels, sample_rate }
    }

    /// 当前可读样本数（0 表示缓冲已空，处于 buffering 状态）
    pub fn readable(&self) -> usize {
        self.consumer.slots()
    }
}

impl Iterator for RingSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        Some(self.consumer.pop().unwrap_or(0.0))
    }
}

impl Source for RingSource {
    fn current_frame_len(&self) -> Option<usize> {
        let readable = self.consumer.slots();
        if readable == 0 {
            None
        } else {
            Some(readable)
        }
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// 全局输出流（保持 OutputStream 存活以维持设备会话）。
///
/// cpal 的 OutputStream 不是 Send/Sync（平台限制），无法放进引擎状态；
/// 把它移入常驻 park 线程持有，引擎侧只保留 Send+Sync 的 OutputStreamHandle。
pub struct Output {
    handle: OutputStreamHandle,
}

impl Output {
    pub fn open() -> Result<Output, String> {
        // cpal 的 OutputStream 不是 Send，无法移入线程，只能在专用线程内创建；
        // 该线程持有 stream 维持设备会话，仅把 Send+Sync 的 handle 传回。
        let (tx, rx) = std::sync::mpsc::channel::<Result<OutputStreamHandle, String>>();
        std::thread::Builder::new()
            .name("audio-output".into())
            .spawn(move || match OutputStream::try_default() {
                Ok((stream, handle)) => {
                    let _keepalive = stream;
                    let _ = tx.send(Ok(handle));
                    loop {
                        std::thread::park();
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("无法打开音频输出设备: {e}")));
                }
            })
            .map_err(|e| format!("音频输出线程启动失败: {e}"))?;
        rx.recv()
            .map_err(|_| "音频输出线程异常退出".to_string())?
            .map(|handle| Output { handle })
    }

    /// 创建一个新 Sink（默认置为暂停，由播放命令显式恢复）
    pub fn new_sink(&self, volume: f32) -> Result<Sink, String> {
        let sink = Sink::try_new(&self.handle).map_err(|e| format!("创建播放器失败: {e}"))?;
        sink.set_volume(volume);
        sink.pause();
        Ok(sink)
    }
}
