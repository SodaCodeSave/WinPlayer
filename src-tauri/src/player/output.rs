//! 音频输出封装：rodio 输出流与 Sink 管理。
//!
//! OutputStream 全局唯一并随引擎存活；每次加载/seek 创建新的 Sink
//! （丢弃旧 Sink 前必须显式 `stop()`，否则混音器里会残留旧音源继续出声）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rodio::{OutputStream, OutputStreamHandle, Sink, Source};

/// 从 ring buffer 读取样本的 rodio Source。
///
/// ring 为空时输出静音（混音器需要它持续拉取以保持设备活跃）；
/// 采样率/声道数声明为解码流的原始值，rodio 混音器会自动转换到设备参数。
///
/// `ended` 置位且 ring 排空后，`next()` 返回 None——rodio Sink 会把该源
/// 从播放队列移除并自动衔接下一个 append 进来的源（Gapless 的衔接机制）。
pub struct RingSource {
    consumer: rtrb::Consumer<f32>,
    channels: u16,
    sample_rate: u32,
    ended: Arc<AtomicBool>,
}

impl RingSource {
    pub fn new(consumer: rtrb::Consumer<f32>, channels: u16, sample_rate: u32) -> Self {
        RingSource {
            consumer,
            channels,
            sample_rate,
            ended: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 结束标记句柄（解码端全部样本推送完毕后由引擎远程置位：
    /// ring 排空后本源即结束，rodio Sink 自动衔接下一源）
    pub fn ended_handle(&self) -> Arc<AtomicBool> {
        self.ended.clone()
    }
}

impl Iterator for RingSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        match self.consumer.pop() {
            Ok(sample) => Some(sample),
            Err(_) => {
                if self.ended.load(Ordering::Acquire) {
                    None
                } else {
                    Some(0.0)
                }
            }
        }
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
///
/// 长时间无音频活动后底层设备会话可能被系统回收（Android 退后台、
/// Windows 睡眠或音频设备切换），此时旧流上的 Sink 不再被回调消费，
/// 必须丢弃旧流重开：`stop` 标志用于唤醒 park 线程退出并释放设备。
pub struct Output {
    handle: OutputStreamHandle,
    stop: Arc<AtomicBool>,
    keepalive: std::thread::JoinHandle<()>,
}

impl Drop for Output {
    fn drop(&mut self) {
        // 通知常驻线程退出：唤醒后释放 OutputStream，设备会话随之关闭
        self.stop.store(true, Ordering::Release);
        self.keepalive.thread().unpark();
    }
}

impl Output {
    pub fn open() -> Result<Output, String> {
        // cpal 的 OutputStream 不是 Send，无法移入线程，只能在专用线程内创建；
        // 该线程持有 stream 维持设备会话，仅把 Send+Sync 的 handle 传回。
        let (tx, rx) = std::sync::mpsc::channel::<Result<OutputStreamHandle, String>>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let keepalive = std::thread::Builder::new()
            .name("audio-output".into())
            .spawn(move || match OutputStream::try_default() {
                Ok((stream, handle)) => {
                    let _keepalive = stream;
                    let _ = tx.send(Ok(handle));
                    // 被 park 唤醒且标志置位时退出（Output 被丢弃 = 重建输出流）
                    while !stop_flag.load(Ordering::Acquire) {
                        std::thread::park();
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("无法打开音频输出设备: {e}")));
                }
            })
            .map_err(|e| format!("音频输出线程启动失败: {e}"))?;
        let handle = rx
            .recv()
            .map_err(|_| "音频输出线程异常退出".to_string())??;
        Ok(Output { handle, stop, keepalive })
    }

    /// 创建一个新 Sink（默认置为暂停，由播放命令显式恢复）
    pub fn new_sink(&self, volume: f32) -> Result<Sink, String> {
        let sink = Sink::try_new(&self.handle).map_err(|e| format!("创建播放器失败: {e}"))?;
        sink.set_volume(volume);
        sink.pause();
        Ok(sink)
    }
}
