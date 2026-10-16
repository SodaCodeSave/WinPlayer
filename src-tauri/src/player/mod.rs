//! 原生音频播放引擎：解码（symphonia）→ EQ（biquad 链）→ ring buffer → rodio 输出。
//!
//! 会话模型：每次 load/stop/seek 都递增单调会话代号，解码线程每个步骤校验
//! 代号，不匹配立即退出并清理临时文件——快速切歌不会残留旧会话。
//!
//! Tauri 事件（前端 `listen`）：
//! - `player://progress`  `{ currentTime, duration }`，解码循环每 250ms
//! - `player://duration`  `{ duration }`（probe 后，估算或精确，秒）
//! - `player://state`     `{ state }`：buffering / playing / paused / idle
//! - `player://ended`     自然播完
//! - `player://error`     `{ message }`（中文，可直接展示）

pub mod decoder;
pub mod eq;
pub mod http_source;
pub mod output;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use eq::EqChain;
use output::Output;

/// 事件名前缀
const EVENT_PROGRESS: &str = "player://progress";
const EVENT_DURATION: &str = "player://duration";
const EVENT_STATE: &str = "player://state";
const EVENT_ENDED: &str = "player://ended";
const EVENT_ERROR: &str = "player://error";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    current_time: f64,
    duration: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DurationPayload {
    duration: f64,
}

#[derive(Serialize, Clone)]
struct StatePayload {
    state: &'static str,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    message: String,
}

pub struct PlayerEngine {
    app: AppHandle,
    output: Mutex<Option<Output>>,
    /// 当前 Sink（None = 空闲）
    sink: Mutex<Option<rodio::Sink>>,
    /// 会话代号，load/stop/seek 均递增
    session: AtomicU64,
    volume: Mutex<f32>,
    /// 待应用的 seek 目标（秒），由解码线程消费
    pending_seek: Mutex<Option<f64>>,
    /// 用户期望的播放状态（与 Sink 的暂停位镜像）
    is_playing: AtomicBool,
    /// 解码是否跟不上（用于 waiting 事件去重）
    buffering: AtomicBool,
    /// 当前加载的 URL（seek 会话重建时需要）
    current_url: Mutex<Option<String>>,
    /// HTTP 客户端（长连接复用）
    client: reqwest::blocking::Client,
    /// 均衡器
    pub eq: EqChain,
}

impl PlayerEngine {
    pub fn new(app: AppHandle) -> Arc<PlayerEngine> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("WinPlayer/1.2")
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        let engine = Arc::new(PlayerEngine {
            app: app.clone(),
            // 输出设备懒初始化（install_sink 首次调用时打开）：
            // 启动阶段不触碰平台音频栈——Android 上 cpal/Oboe 初始化链路
            // 含大量 unwrap，且 release 配置 panic=abort，一旦 panic 即闪退
            output: Mutex::new(None),
            sink: Mutex::new(None),
            session: AtomicU64::new(0),
            volume: Mutex::new(0.8),
            pending_seek: Mutex::new(None),
            is_playing: AtomicBool::new(false),
            buffering: AtomicBool::new(false),
            current_url: Mutex::new(None),
            client,
            eq: EqChain::new(),
        });
        // 空闲状态广播一次，前端据此复位
        let _ = app.emit(EVENT_STATE, StatePayload { state: "idle" });
        engine
    }

    fn session(&self) -> u64 {
        self.session.load(Ordering::Acquire)
    }

    fn bump_session(&self) -> u64 {
        self.session.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub(crate) fn emit_state(&self, state: &'static str) {
        let _ = self.app.emit(EVENT_STATE, StatePayload { state });
    }

    /// 用新音源整体替换 Sink（旧 Sink 先 stop 再丢弃，防止混音器残留）
    fn install_sink(&self, source: output::RingSource) -> Result<(), String> {
        let output_guard = {
            let mut guard = self.output.lock().unwrap();
            if guard.is_none() {
                // 懒初始化：首次播放才打开音频输出；失败不缓存，下次再试
                *guard = Output::open().ok();
            }
            guard
        };
        let Some(output) = output_guard.as_ref() else {
            return Err("音频输出设备不可用".to_string());
        };
        let sink = output.new_sink(*self.volume.lock().unwrap())?;
        if self.is_playing.load(Ordering::Acquire) {
            sink.play();
        }
        sink.append(source);
        let mut guard = self.sink.lock().unwrap();
        if let Some(old) = guard.take() {
            old.stop();
        }
        *guard = Some(sink);
        Ok(())
    }

    /// 加载 URL 并开始缓冲（不自动播放）
    fn load(self: &Arc<Self>, url: String) {
        self.bump_session();
        *self.pending_seek.lock().unwrap() = None;
        self.buffering.store(false, Ordering::Release);
        self.is_playing.store(false, Ordering::Release);
        *self.current_url.lock().unwrap() = Some(url.clone());
        // 立即清掉旧 Sink（旧音频静音），新 Sink 由解码线程装配
        if let Some(old) = self.sink.lock().unwrap().take() {
            old.stop();
        }
        self.emit_state("buffering");
        decoder::spawn_session(self.clone(), self.session(), url);
    }

    fn stop(self: &Arc<Self>) {
        self.bump_session();
        *self.pending_seek.lock().unwrap() = None;
        self.buffering.store(false, Ordering::Release);
        self.is_playing.store(false, Ordering::Release);
        *self.current_url.lock().unwrap() = None;
        if let Some(old) = self.sink.lock().unwrap().take() {
            old.stop();
        }
        self.emit_state("idle");
    }
}

/* ------------------------------ Tauri commands ------------------------------ */

#[tauri::command]
pub fn player_load(engine: State<Arc<PlayerEngine>>, url: String) {
    engine.load(url);
}

#[tauri::command]
pub fn player_play(engine: State<Arc<PlayerEngine>>) {
    engine.is_playing.store(true, Ordering::Release);
    if let Some(sink) = engine.sink.lock().unwrap().as_ref() {
        sink.play();
    }
    engine.emit_state("playing");
}

#[tauri::command]
pub fn player_pause(engine: State<Arc<PlayerEngine>>) {
    engine.is_playing.store(false, Ordering::Release);
    if let Some(sink) = engine.sink.lock().unwrap().as_ref() {
        sink.pause();
    }
    engine.emit_state("paused");
}

#[tauri::command]
pub fn player_stop(engine: State<Arc<PlayerEngine>>) {
    engine.stop();
}

#[tauri::command]
pub fn player_seek(engine: State<Arc<PlayerEngine>>, seconds: f64) {
    *engine.pending_seek.lock().unwrap() = Some(seconds.max(0.0));
    // seek 后必然继续播（与 HTML5 行为一致），唤醒 Sink
    engine.is_playing.store(true, Ordering::Release);
    if let Some(sink) = engine.sink.lock().unwrap().as_ref() {
        sink.play();
    }
}

#[tauri::command]
pub fn player_set_volume(engine: State<Arc<PlayerEngine>>, volume: f32) {
    let volume = volume.clamp(0.0, 1.0);
    *engine.volume.lock().unwrap() = volume;
    if let Some(sink) = engine.sink.lock().unwrap().as_ref() {
        sink.set_volume(volume);
    }
}

#[tauri::command]
pub fn player_set_eq(engine: State<Arc<PlayerEngine>>, enabled: bool, gains: Vec<f32>) {
    let mut bands = [0.0f32; 10];
    for (i, value) in gains.iter().take(10).enumerate() {
        bands[i] = value.clamp(-12.0, 12.0);
    }
    engine.eq.update(|state| {
        state.set_enabled(enabled);
        state.set_gains(bands);
    });
}

/// setup 阶段创建引擎并挂到应用状态
pub fn init(app: &tauri::App) {
    let engine = PlayerEngine::new(app.handle().clone());
    app.manage(engine);
}

/* ------------------------------ 供解码线程使用的辅助 ------------------------------ */

impl PlayerEngine {
    pub(crate) fn client(&self) -> reqwest::blocking::Client {
        self.client.clone()
    }

    pub(crate) fn buffer_dir(&self) -> Option<std::path::PathBuf> {
        self.app.path().app_cache_dir().ok()
    }

    pub(crate) fn is_current_session(&self, session: u64) -> bool {
        self.session() == session
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Acquire)
    }

    pub(crate) fn take_pending_seek(&self) -> Option<f64> {
        self.pending_seek.lock().unwrap().take()
    }

    /// 是否有待应用的 seek（只窥探不消费，供 push_all 中断判断）
    pub(crate) fn has_pending_seek(&self) -> bool {
        self.pending_seek.lock().unwrap().is_some()
    }

    /// 换装新 ring（seek 后调用）：整体替换 Sink 以瞬时切断旧数据
    pub(crate) fn swap_sink(&self, source: output::RingSource) -> Result<(), String> {
        self.install_sink(source)
    }

    pub(crate) fn set_buffering(&self, value: bool) -> bool {
        self.buffering.swap(value, Ordering::AcqRel)
    }

    pub(crate) fn emit_progress(&self, current_time: f64, duration: f64) {
        let _ = self
            .app
            .emit(EVENT_PROGRESS, ProgressPayload { current_time, duration });
    }

    pub(crate) fn emit_duration(&self, duration: f64) {
        let _ = self.app.emit(EVENT_DURATION, DurationPayload { duration });
    }

    pub(crate) fn emit_ended(&self) {
        self.is_playing.store(false, Ordering::Release);
        self.buffering.store(false, Ordering::Release);
        let _ = self.app.emit(EVENT_ENDED, ());
    }

    pub(crate) fn emit_error(&self, message: String) {
        self.is_playing.store(false, Ordering::Release);
        self.buffering.store(false, Ordering::Release);
        let _ = self.app.emit(EVENT_ERROR, ErrorPayload { message });
    }
}
