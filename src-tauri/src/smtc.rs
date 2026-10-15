//! Windows 系统媒体传输控件（SMTC，System Media Transport Controls）集成。
//!
//! Win32 桌面应用不能用 `SystemMediaTransportControls::GetForCurrentView()`（那是 UWP 专属），
//! 官方提供的接入方式是 `ISystemMediaTransportControlsInterop::GetForWindow`，
//! 用主窗口的 HWND 换取归属于本进程的 `SystemMediaTransportControls`。接入后：
//!
//! - 歌曲标题 / 歌手 / 专辑 / 封面会出现在 Windows 媒体浮出窗、锁屏与蓝牙设备上
//! - 播放进度（位置 / 时长）同步给系统，浮出窗展示时间轴
//! - 系统侧的播放、暂停、上一首、下一首按钮事件被转发回前端，由播放器执行
//!
//! 前端每次把播放器快照（`SmtcSnapshot`）整体同步过来，Rust 侧 diff 后只调用真正变化的
//! WinRT 属性，避免进度更新（约每秒一次）反复重建媒体信息。

use serde::{Deserialize, Serialize};

/// Rust → 前端的 SMTC 按钮事件名，payload 为按钮动作字符串
pub const SMTC_BUTTON_EVENT: &str = "smtc-button";

/// 同步给 SMTC 的播放状态
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmtcStatus {
    Playing,
    Paused,
    Stopped,
}

/// 前端同步上来的播放器快照（整体状态，而非增量）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtcSnapshot {
    /// 是否存在已加载的曲目；为 false 时隐藏系统媒体控件
    pub active: bool,
    pub status: SmtcStatus,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// 封面地址（http / https / file），为空表示不展示封面
    pub cover: Option<String>,
    /// 播放位置（毫秒）
    pub position_ms: u64,
    /// 总时长（毫秒）
    pub duration_ms: u64,
}

#[cfg(windows)]
mod imp {
    use std::sync::{Arc, Mutex};

    use tauri::{AppHandle, Emitter, Manager};
    use windows::core::{HSTRING, Ref, Result as WinResult};
    use windows::Foundation::{TimeSpan, TypedEventHandler, Uri};
    use windows::Media::{
        MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
        SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
        SystemMediaTransportControlsTimelineProperties,
    };
    use windows::Storage::Streams::RandomAccessStreamReference;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::WinRT::{
        ISystemMediaTransportControlsInterop, RoGetActivationFactory, RoInitialize, RO_INIT_MULTITHREADED,
    };

    use super::{SmtcSnapshot, SmtcStatus, SMTC_BUTTON_EVENT};

    /// 按钮事件回传通道（生产环境转发给前端，测试里用于断言）
    type ButtonSink = Arc<dyn Fn(&'static str) + Send + Sync + 'static>;

    /// SMTC 槽位：初始化失败时为 `None`，之后的同步调用静默跳过
    struct SmtcSlot(Mutex<Option<Smtc>>);

    struct Smtc {
        controls: SystemMediaTransportControls,
        button_token: i64,
        /// 当前会话是否对外可见
        active: bool,
        /// 最近一次写入 DisplayUpdater 的媒体信息指纹
        media_key: String,
        /// 最近一次写入的播放状态（i32：MediaPlaybackStatus 的值）
        status: Option<i32>,
        /// 最近一次写入的 (位置, 时长)，单位 100 纳秒
        timeline: Option<(i64, i64)>,
    }

    /// 保证当前线程处于可用的 WinRT apartment。
    /// 线程已初始化为其它 apartment 时返回 `RPC_E_CHANGED_MODE`，此时同样可用，忽略即可。
    fn ensure_winrt() {
        unsafe {
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
        }
    }

    /// 毫秒 → WinRT TimeSpan（100 纳秒刻度）
    fn millis(value: u64) -> TimeSpan {
        TimeSpan {
            Duration: value as i64 * 10_000,
        }
    }

    fn button_action(button: SystemMediaTransportControlsButton) -> Option<&'static str> {
        match button {
            SystemMediaTransportControlsButton::Play => Some("play"),
            SystemMediaTransportControlsButton::Pause => Some("pause"),
            SystemMediaTransportControlsButton::Stop => Some("stop"),
            SystemMediaTransportControlsButton::Next => Some("next"),
            SystemMediaTransportControlsButton::Previous => Some("previous"),
            _ => None,
        }
    }

    fn media_key_of(snapshot: &SmtcSnapshot) -> String {
        format!(
            "{}\u{1}{}\u{1}{}\u{1}{}",
            snapshot.title,
            snapshot.artist,
            snapshot.album,
            snapshot.cover.as_deref().unwrap_or_default()
        )
    }

    impl Smtc {
        fn create(hwnd: HWND, sink: ButtonSink) -> WinResult<Self> {
            ensure_winrt();

            let interop: ISystemMediaTransportControlsInterop = unsafe {
                RoGetActivationFactory(&HSTRING::from(
                    "Windows.Media.SystemMediaTransportControls",
                ))?
            };
            let controls: SystemMediaTransportControls = unsafe { interop.GetForWindow(hwnd)? };

            // 只开放播放器真正能处理的按钮，避免系统展示无响应的控件
            controls.SetIsPlayEnabled(true)?;
            controls.SetIsPauseEnabled(true)?;
            controls.SetIsNextEnabled(true)?;
            controls.SetIsPreviousEnabled(true)?;
            controls.SetIsStopEnabled(false)?;
            controls.SetIsRecordEnabled(false)?;
            controls.SetIsFastForwardEnabled(false)?;
            controls.SetIsRewindEnabled(false)?;
            controls.SetIsChannelUpEnabled(false)?;
            controls.SetIsChannelDownEnabled(false)?;
            // 还没有曲目时先把会话藏起来，等首次同步再启用
            controls.SetIsEnabled(false)?;

            let on_button = TypedEventHandler::new(
                move |_sender: Ref<'_, SystemMediaTransportControls>,
                      args: Ref<'_, SystemMediaTransportControlsButtonPressedEventArgs>| {
                    if let Some(event) = args.as_ref() {
                        if let Ok(button) = event.Button() {
                            if let Some(action) = button_action(button) {
                                sink(action);
                            }
                        }
                    }
                    Ok(())
                },
            );
            let button_token = controls.ButtonPressed(&on_button)?;

            Ok(Self {
                controls,
                button_token,
                active: false,
                media_key: String::new(),
                status: None,
                timeline: None,
            })
        }

        fn apply(&mut self, snapshot: &SmtcSnapshot) -> WinResult<()> {
            if !snapshot.active {
                if self.active {
                    self.controls.SetIsEnabled(false)?;
                    self.controls.SetPlaybackStatus(MediaPlaybackStatus::Closed)?;
                    self.active = false;
                    self.media_key.clear();
                    self.status = None;
                    self.timeline = None;
                }
                return Ok(());
            }

            // 先补齐媒体信息与状态，再露出会话，避免浮出窗短暂显示上一首的残留信息
            let media_key = media_key_of(snapshot);
            if !self.active || media_key != self.media_key {
                self.update_display(snapshot)?;
                self.media_key = media_key;
            }

            let status = match snapshot.status {
                SmtcStatus::Playing => MediaPlaybackStatus::Playing,
                SmtcStatus::Paused => MediaPlaybackStatus::Paused,
                SmtcStatus::Stopped => MediaPlaybackStatus::Stopped,
            };
            if self.status != Some(status.0) {
                self.controls.SetPlaybackStatus(status)?;
                self.status = Some(status.0);
            }

            let position = millis(snapshot.position_ms).Duration;
            let duration = millis(snapshot.duration_ms.max(snapshot.position_ms)).Duration;
            if self.timeline != Some((position, duration)) {
                let props = SystemMediaTransportControlsTimelineProperties::new()?;
                props.SetStartTime(TimeSpan { Duration: 0 })?;
                props.SetMinSeekTime(TimeSpan { Duration: 0 })?;
                props.SetPosition(TimeSpan { Duration: position })?;
                props.SetMaxSeekTime(TimeSpan { Duration: duration })?;
                self.controls.UpdateTimelineProperties(&props)?;
                self.timeline = Some((position, duration));
            }

            if !self.active {
                self.controls.SetIsEnabled(true)?;
                self.active = true;
            }
            Ok(())
        }

        fn update_display(&self, snapshot: &SmtcSnapshot) -> WinResult<()> {
            let updater = self.controls.DisplayUpdater()?;
            updater.SetType(MediaPlaybackType::Music)?;
            let music = updater.MusicProperties()?;
            music.SetTitle(&HSTRING::from(&snapshot.title))?;
            music.SetArtist(&HSTRING::from(&snapshot.artist))?;
            music.SetAlbumTitle(&HSTRING::from(&snapshot.album))?;

            match snapshot
                .cover
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                Some(url) => {
                    // 封面交给系统按 URI 自行拉取；地址非法时退化为无封面
                    if let Ok(uri) = Uri::CreateUri(&HSTRING::from(url)) {
                        if let Ok(thumbnail) = RandomAccessStreamReference::CreateFromUri(&uri) {
                            let _ = updater.SetThumbnail(&thumbnail);
                        }
                    }
                }
                None => {
                    let _ = updater.SetThumbnail(None::<&RandomAccessStreamReference>);
                }
            }

            updater.Update()?;
            Ok(())
        }
    }

    impl Drop for Smtc {
        fn drop(&mut self) {
            let _ = self.controls.RemoveButtonPressed(self.button_token);
        }
    }

    fn create_for_app(app: &AppHandle, hwnd: HWND) -> WinResult<Smtc> {
        let handle = app.clone();
        let sink: ButtonSink = Arc::new(move |action| {
            let _ = handle.emit(SMTC_BUTTON_EVENT, action);
        });
        Smtc::create(hwnd, sink)
    }

    /// 在应用启动（setup）时调用：创建 SMTC 并挂到 Tauri 状态里。
    /// 任何失败都只记录日志，不中断应用启动。
    pub fn init(app: &AppHandle) {
        ensure_winrt();

        let created = app
            .get_webview_window("main")
            .ok_or_else(|| "未找到主窗口".to_string())
            .and_then(|window| window.hwnd().map_err(|error| error.to_string()))
            .and_then(|hwnd| create_for_app(app, hwnd).map_err(|error| error.to_string()));

        if let Err(error) = &created {
            eprintln!("[smtc] 系统媒体控件不可用：{error}");
        }
        app.manage(SmtcSlot(Mutex::new(created.ok())));
    }

    /// 把播放器快照同步到系统媒体控件（由前端 `smtc_sync` 命令触发）
    pub fn sync(app: &AppHandle, snapshot: &SmtcSnapshot) -> Result<(), String> {
        ensure_winrt();

        let Some(slot) = app.try_state::<SmtcSlot>() else {
            return Ok(());
        };
        let mut guard = slot.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(smtc) = guard.as_mut() else {
            return Ok(());
        };
        smtc.apply(snapshot).map_err(|error| error.to_string())
    }

    #[cfg(test)]
    mod tests {
        use std::sync::{Arc, Mutex as StdMutex};
        use std::time::{Duration, Instant};

        use windows::core::{w, Result as WinResult};
        use windows::Media::Control::{
            GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
            WS_OVERLAPPED, WINDOW_EX_STYLE,
        };

        use super::{ensure_winrt, ButtonSink, Smtc};
        use crate::smtc::{SmtcSnapshot, SmtcStatus};

        const TEST_TITLE: &str = "WinPlayer SMTC Smoke Test";

        /// 空转一小段时间并泵消息（MTA 下事件走线程池，泵消息只是稳妥起见）
        fn pump(duration: Duration) {
            let deadline = Instant::now() + duration;
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while Instant::now() < deadline {
                unsafe {
                    while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        /// 等待异步操作完成并取结果（IAsyncOperation 未完成时 GetResults 返回错误）
        fn wait_result<T>(op: &impl Fn() -> WinResult<T>, timeout: Duration) -> T {
            let deadline = Instant::now() + timeout;
            loop {
                if let Ok(value) = op() {
                    return value;
                }
                assert!(Instant::now() < deadline, "等待 SMTC 异步操作超时");
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        /// 在全局会话管理器里按标题找本进程发布的会话
        fn find_session(title: &str, timeout: Duration) -> GlobalSystemMediaTransportControlsSession {
            let manager_op =
                GlobalSystemMediaTransportControlsSessionManager::RequestAsync().expect("请求 GSMTC 管理器失败");
            let manager = wait_result(&|| manager_op.GetResults(), Duration::from_secs(10));

            let deadline = Instant::now() + timeout;
            loop {
                let sessions = manager.GetSessions().expect("获取会话列表失败");
                for index in 0..sessions.Size().expect("读取会话数量失败") {
                    let session = sessions.GetAt(index).expect("读取会话失败");
                    let props_op = session.TryGetMediaPropertiesAsync().expect("读取媒体信息失败");
                    let props = wait_result(&|| props_op.GetResults(), Duration::from_secs(5));
                    let got_title = props.Title().map(|value| value.to_string()).unwrap_or_default();
                    if got_title == title {
                        return session;
                    }
                }
                assert!(
                    Instant::now() < deadline,
                    "GSMTC 中未找到标题为 {title:?} 的媒体会话"
                );
                std::thread::sleep(Duration::from_millis(200));
            }
        }

        #[test]
        fn smtc_publishes_session_and_receives_buttons() {
            ensure_winrt();

            let hwnd = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("WinPlayer SMTC Test Window"),
                    WS_OVERLAPPED,
                    0,
                    0,
                    320,
                    240,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .expect("创建测试窗口失败");

            let received: Arc<StdMutex<Vec<&'static str>>> = Arc::new(StdMutex::new(Vec::new()));
            let sink: ButtonSink = {
                let log = received.clone();
                Arc::new(move |action| log.lock().unwrap().push(action))
            };

            let mut smtc = Smtc::create(hwnd, sink).expect("创建 SMTC 失败");
            let snapshot = SmtcSnapshot {
                active: true,
                status: SmtcStatus::Playing,
                title: TEST_TITLE.to_string(),
                artist: "WinPlayer".to_string(),
                album: "Smoke Test".to_string(),
                cover: None,
                position_ms: 30_000,
                duration_ms: 200_000,
            };
            smtc.apply(&snapshot).expect("同步播放状态失败");

            // 1. 系统里应该出现本进程发布的媒体会话
            let session = find_session(TEST_TITLE, Duration::from_secs(10));
            println!(
                "[smtc] 会话 AUMID = {}",
                session.SourceAppUserModelId().unwrap().to_string()
            );

            // 2. 播放状态
            let playback = session.GetPlaybackInfo().expect("读取 PlaybackInfo 失败");
            let status = playback.PlaybackStatus().expect("读取 PlaybackStatus 失败");
            println!("[smtc] PlaybackStatus = {status:?}");
            assert_eq!(
                status,
                GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
            );

            // 3. 时间轴
            let timeline = session.GetTimelineProperties().expect("读取时间轴失败");
            let position = timeline.Position().unwrap().Duration;
            let max = timeline.MaxSeekTime().unwrap().Duration;
            println!(
                "[smtc] 位置 = {} ms / 上限 = {} ms",
                position / 10_000,
                max / 10_000
            );
            assert_eq!(position, 30_000 * 10_000);
            assert_eq!(max, 200_000 * 10_000);

            // 4. 反向链路：系统发起暂停 → 应用收到 ButtonPressed
            let pause_op = session.TryPauseAsync().expect("TryPauseAsync 失败");
            wait_result(&|| pause_op.GetResults(), Duration::from_secs(5));
            pump(Duration::from_secs(3));
            let actions = received.lock().unwrap().clone();
            println!("[smtc] 收到的按钮事件 = {actions:?}");
            assert!(actions.contains(&"pause"), "未收到 SMTC 的 pause 按钮事件");

            // 清理：隐藏会话并销毁测试窗口
            smtc.apply(&SmtcSnapshot {
                active: false,
                status: SmtcStatus::Stopped,
                title: String::new(),
                artist: String::new(),
                album: String::new(),
                cover: None,
                position_ms: 0,
                duration_ms: 0,
            })
            .expect("隐藏 SMTC 失败");
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use tauri::AppHandle;

    use super::SmtcSnapshot;

    pub fn init(_app: &AppHandle) {}

    pub fn sync(_app: &AppHandle, _snapshot: &SmtcSnapshot) -> Result<(), String> {
        Ok(())
    }
}

pub use imp::{init, sync};

#[cfg(test)]
mod tests {
    use super::SmtcSnapshot;

    /// 前端（services/smtc.ts）与 Rust 之间的载荷契约：字段名经 serde 转为 camelCase
    #[test]
    fn snapshot_deserializes_from_frontend_payload() {
        let json = r#"{
            "active": true,
            "status": "playing",
            "title": "晴天",
            "artist": "周杰伦 / 费玉清",
            "album": "叶惠美",
            "cover": "https://example.com/cover.jpg",
            "positionMs": 1234,
            "durationMs": 240000
        }"#;
        let snapshot: SmtcSnapshot =
            serde_json::from_str(json).expect("前端载荷应能反序列化为 SmtcSnapshot");
        assert!(snapshot.active);
        assert_eq!(snapshot.title, "晴天");
        assert_eq!(snapshot.artist, "周杰伦 / 费玉清");
        assert_eq!(snapshot.cover.as_deref(), Some("https://example.com/cover.jpg"));
        assert_eq!(snapshot.position_ms, 1234);
        assert_eq!(snapshot.duration_ms, 240000);

        // status 是小写枚举
        let paused = r#"{"active":true,"status":"paused","title":"","artist":"","album":"","cover":null,"positionMs":0,"durationMs":0}"#;
        let snapshot: SmtcSnapshot =
            serde_json::from_str(paused).expect("status 应接受小写取值");
        assert_eq!(snapshot.status, super::SmtcStatus::Paused);
    }
}
