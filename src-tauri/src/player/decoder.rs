//! 解码线程：symphonia probe → 解码为 f32 交错样本 → EQ → ring buffer。
//!
//! 采样率/声道转换交给 rodio 混音器（UniformSourceIterator 自动把
//! RingSource 声明的原始参数转换到设备参数），解码线程不做重采样。
//!
//! 线程随时可能因会话代号变化而退出（快速切歌/停止），所有副作用
//! （Sink 装配、事件广播）都以「仍是当前会话」为前提。

use std::sync::Arc;
use std::time::{Duration, Instant};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use super::http_source::HttpMediaSource;
use super::output::RingSource;
use super::PlayerEngine;

/// progress 事件的发射间隔
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// ring buffer 容量（样本数）：按 1 秒估算，下限 65536
fn ring_capacity(sample_rate: u32, channels: usize) -> usize {
    (sample_rate as usize * channels.max(1)).max(65_536)
}

/// 从 URL 提取扩展名作为 probe 提示
fn extension_hint(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let name = path.rsplit('/').next()?;
    let ext = name.rsplit('.').next()?;
    if !ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(ext.to_ascii_lowercase())
    } else {
        None
    }
}

/// 启动解码会话线程（engine.load 调用）
pub(crate) fn spawn_session(engine: Arc<PlayerEngine>, session: u64, url: String) {
    std::thread::Builder::new()
        .name("audio-decode".into())
        .spawn(move || {
            let outcome = run_session(&engine, session, &url);
            // 仅当前会话才向外广播结果
            if engine.is_current_session(session) {
                match outcome {
                    Ok(SessionOutcome::Ended) => engine.emit_ended(),
                    Ok(SessionOutcome::Superseded) => {}
                    Err(message) => engine.emit_error(message),
                }
            }
        })
        .ok();
}

enum SessionOutcome {
    /// 自然播完（ring 已排空）
    Ended,
    /// 被新会话取代（静默退出）
    Superseded,
}

fn run_session(engine: &Arc<PlayerEngine>, session: u64, url: &str) -> Result<SessionOutcome, String> {
    let dir = engine
        .buffer_dir()
        .ok_or_else(|| "无法定位音频缓冲目录".to_string())?;

    // 创建流式源（同步等待首块数据，保证 probe 立即有数据可读）
    let source = HttpMediaSource::new(engine.client(), url.to_string(), &dir)
        .map_err(|e| format!("加载音频失败: {e}"))?;
    if !engine.is_current_session(session) {
        return Ok(SessionOutcome::Superseded);
    }

    // 退出时清理缓冲临时文件（结束/错误/被取代都经过这里）
    let _cleanup = TempFileGuard(source.temp_path());

    let mut hint = Hint::new();
    if let Some(ext) = extension_hint(url) {
        hint.with_extension(&ext);
    }

    let mss = MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(describe_probe_error)?;
    if !engine.is_current_session(session) {
        return Ok(SessionOutcome::Superseded);
    }

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| "音频文件中没有可播放的轨道".to_string())?;
    let track_id = track.id;
    let time_base = track.codec_params.time_base;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(describe_codec_error)?;

    // 总时长：容器/轨道元数据（n_frames × time_base）优先，退回按总字节数 × 平均码率估算
    let mut duration_secs = match (track.codec_params.n_frames, time_base) {
        (Some(n_frames), Some(tb)) => n_frames as f64 * tb.numer as f64 / tb.denom as f64,
        _ => 0.0,
    };
    if duration_secs > 0.0 {
        engine.emit_duration(duration_secs);
    }

    // 首帧解码前采样率/声道未知，ring 延迟创建
    let mut ring: Option<(rtrb::Producer<f32>, usize)> = None;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut channels = 0usize;
    let mut sample_rate = 0u32;

    let mut current_time = 0.0f64;
    let mut last_progress = Instant::now();
    let mut was_buffering = false;

    loop {
        if !engine.is_current_session(session) {
            return Ok(SessionOutcome::Superseded);
        }

        // 处理 seek 请求（可能在 push 阻塞期间被设置）
        if let Some(secs) = engine.take_pending_seek() {
            apply_seek(
                engine,
                session,
                &mut format,
                &mut decoder,
                track_id,
                secs,
                channels,
                sample_rate,
                &mut ring,
                &mut sample_buf,
                &mut current_time,
                &mut was_buffering,
                duration_secs,
            )?;
            last_progress = Instant::now();
            continue;
        }

        // 读取下一包
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                // 自然结束：MP3 无 Xing 头等场景到结尾才知道真实时长
                if duration_secs <= 0.0 && current_time > 0.0 {
                    duration_secs = current_time;
                    engine.emit_duration(duration_secs);
                    engine.emit_progress(current_time, duration_secs);
                }
                // 等 ring 排空（会话变更时提前返回）
                if let Some((producer, capacity)) = ring.as_ref() {
                    while engine.is_current_session(session) && producer.slots() < *capacity {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
                if !engine.is_current_session(session) {
                    return Ok(SessionOutcome::Superseded);
                }
                return Ok(SessionOutcome::Ended);
            }
            Err(SymphoniaError::ResetRequired) => return Err("音频流被重置".to_string()),
            Err(e) => return Err(format!("解码出错: {e}")),
        };

        if let Some(tb) = time_base {
            current_time = packet.ts as f64 * tb.numer as f64 / tb.denom as f64;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue, // 单包损坏：跳过
            Err(e) => return Err(describe_codec_error(e)),
        };

        // 首帧：确定流参数并装配 ring + Sink
        if ring.is_none() {
            let spec = *decoded.spec();
            channels = spec.channels.count();
            sample_rate = spec.rate;
            engine.eq.update(|state| state.set_stream_format(sample_rate, channels));

            let capacity = ring_capacity(sample_rate, channels);
            let (producer, consumer) = rtrb::RingBuffer::new(capacity);
            let ring_source = RingSource::new(consumer, channels as u16, sample_rate);
            engine.swap_sink(ring_source)?;
            ring = Some((producer, capacity));

            let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
            buffer.copy_interleaved_ref(decoded);
            match push_all(engine, session, &mut ring, &buffer.samples()) {
                PushOutcome::Superseded => return Ok(SessionOutcome::Superseded),
                // 首帧期间收到 seek：回到主循环顶部统一应用
                PushOutcome::SeekRequested => continue,
                PushOutcome::Completed => {}
            }
            if !engine.is_current_session(session) {
                return Ok(SessionOutcome::Superseded);
            }
            last_progress = Instant::now();
            engine.emit_progress(current_time, duration_secs);
            continue;
        }

        // buffering 检测：播放中且 ring 已空 → 广播 buffering；恢复后广播 playing
        let buffering_now = engine.is_playing() && ring_full(&ring);
        if buffering_now && !was_buffering {
            was_buffering = true;
            engine.set_buffering(true);
            engine.emit_state("buffering");
        } else if !buffering_now && was_buffering {
            was_buffering = false;
            engine.set_buffering(false);
            engine.emit_state("playing");
        }

        let spec = *decoded.spec();
        if sample_buf.is_none() {
            sample_buf = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
        }
        let buffer = sample_buf.as_mut().unwrap();
        buffer.copy_interleaved_ref(decoded);
        match push_all(engine, session, &mut ring, buffer.samples()) {
            PushOutcome::Superseded => return Ok(SessionOutcome::Superseded),
            // ring 满阻塞期间收到 seek：样本丢弃，回到主循环顶部应用
            PushOutcome::SeekRequested => continue,
            PushOutcome::Completed => {}
        }
        if !engine.is_current_session(session) {
            return Ok(SessionOutcome::Superseded);
        }

        // progress 事件（节流）
        if last_progress.elapsed() >= PROGRESS_INTERVAL {
            last_progress = Instant::now();
            engine.emit_progress(current_time, duration_secs);
        }
    }
}

/// push_all 的结果：ring 满阻塞期间可能发生会话变更或收到新 seek
enum PushOutcome {
    /// 全部推完
    Completed,
    /// 会话已被取代（静默退出）
    Superseded,
    /// 期间收到新的 seek 请求（样本已丢弃，交由主循环统一应用）
    SeekRequested,
}

/// 把样本推入 ring（阻塞直到推完）；期间发生会话变更/新 seek 时中断（样本丢弃）
fn push_all(
    engine: &Arc<PlayerEngine>,
    session: u64,
    ring: &mut Option<(rtrb::Producer<f32>, usize)>,
    samples: &[f32],
) -> PushOutcome {
    let Some((producer, _)) = ring.as_mut() else {
        return PushOutcome::Completed;
    };
    let mut offset = 0usize;
    while offset < samples.len() {
        match producer.push(samples[offset]) {
            Ok(()) => offset += 1,
            Err(_) => {
                // ring 已满：检查是否需要中断，否则等待消费端腾出空间
                if !engine.is_current_session(session) {
                    return PushOutcome::Superseded;
                }
                // 只窥探不取走：seek 值由主循环顶部统一取出应用
                if engine.has_pending_seek() {
                    return PushOutcome::SeekRequested;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
    PushOutcome::Completed
}

fn ring_full(ring: &Option<(rtrb::Producer<f32>, usize)>) -> bool {
    match ring {
        Some((producer, capacity)) => producer.slots() == *capacity,
        None => true,
    }
}

/// 应用 seek：symphonia 时间寻址 + 重置解码器 + 换新 ring/Sink
#[allow(clippy::too_many_arguments)]
fn apply_seek(
    engine: &Arc<PlayerEngine>,
    session: u64,
    format: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    secs: f64,
    channels: usize,
    sample_rate: u32,
    ring: &mut Option<(rtrb::Producer<f32>, usize)>,
    sample_buf: &mut Option<SampleBuffer<f32>>,
    current_time: &mut f64,
    was_buffering: &mut bool,
    duration_secs: f64,
) -> Result<(), String> {
    let target = Time::from(secs);
    if let Err(_) = format.seek(SeekMode::Coarse, SeekTo::Time { time: target, track_id: Some(track_id) }) {
        return Err("该音源不支持拖动进度".to_string());
    }
    decoder.reset();
    sample_buf.take();

    // 换新 ring + Sink，瞬时切断旧数据
    let capacity = ring_capacity(sample_rate, channels);
    let (producer, consumer) = rtrb::RingBuffer::new(capacity);
    let ring_source = RingSource::new(consumer, channels.max(1) as u16, if sample_rate > 0 { sample_rate } else { 44_100 });
    engine.swap_sink(ring_source)?;
    *ring = Some((producer, capacity));
    *was_buffering = engine.is_playing();
    if *was_buffering {
        engine.emit_state("buffering");
    }
    *current_time = secs;
    engine.emit_progress(*current_time, duration_secs);
    let _ = session;
    Ok(())
}

fn describe_probe_error(e: SymphoniaError) -> String {
    match e {
        SymphoniaError::Unsupported(_) => "音频格式不支持（可能是 OPUS 等暂不支持的编码）".to_string(),
        SymphoniaError::IoError(err) => format!("读取音频数据失败: {err}"),
        _ => format!("无法识别音频格式: {e}"),
    }
}

fn describe_codec_error(e: SymphoniaError) -> String {
    match e {
        SymphoniaError::Unsupported(_) => "音频编码不支持".to_string(),
        SymphoniaError::IoError(err) => format!("读取音频数据失败: {err}"),
        _ => format!("解码失败: {e}"),
    }
}

/// 退出作用域时删除临时缓冲文件
struct TempFileGuard(std::path::PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
