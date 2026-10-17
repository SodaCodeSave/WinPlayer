use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

#[cfg(mobile)]
use tauri::Manager;

#[cfg(mobile)]
use serde::Serialize;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.tauri.mediasession";

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_media_session);

/// Media playback state.
///
/// All fields are optional — omitted fields preserve their previous values
/// on the native side (merge semantics).
#[cfg(mobile)]
#[derive(Debug, Default, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    /// Track title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Artist name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    /// Album name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    /// URL to an image (JPEG/PNG). Downloaded natively (no CORS).
    /// Use this when the image is on a CDN or external server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artwork_url: Option<String>,
    /// Track duration in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Current playback position in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<f64>,
    /// Playback speed multiplier (default: 1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback_speed: Option<f64>,
    /// Whether media is currently playing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_playing: Option<bool>,
    /// Whether the "previous track" action is available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_prev: Option<bool>,
    /// Whether the "next track" action is available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_next: Option<bool>,
    /// Whether seeking is available (default: true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_seek: Option<bool>,
}

/// Lightweight timeline update (position, duration, speed only).
///
/// Skips notification rebuild — ideal for frequent position syncs.
#[cfg(mobile)]
#[derive(Debug, Default, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineUpdate {
    /// Current playback position in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<f64>,
    /// Track duration in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Playback speed multiplier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback_speed: Option<f64>,
}

/// Handle to the native media session.
#[cfg(mobile)]
pub struct MediaSession<R: Runtime>(tauri::plugin::PluginHandle<R>);

#[cfg(mobile)]
impl<R: Runtime> MediaSession<R> {
    /// Update the media session state and notification.
    ///
    /// Auto-initializes the session on first call.
    /// Only include the fields that changed — previous values are preserved.
    pub fn update_state(&self, state: MediaState) -> Result<(), String> {
        self.0
            .run_mobile_plugin("updateState", state)
            .map_err(|e| format!("{e}"))
    }

    /// Lightweight timeline update — only touches `PlaybackState`, skips notification rebuild.
    ///
    /// Use this for frequent position syncs during playback.
    /// The session must already be initialized via [`update_state`](Self::update_state).
    pub fn update_timeline(&self, timeline: TimelineUpdate) -> Result<(), String> {
        self.0
            .run_mobile_plugin("updateTimeline", timeline)
            .map_err(|e| format!("{e}"))
    }

    /// Clear the media session, dismiss the notification, and release resources.
    pub fn clear(&self) -> Result<(), String> {
        self.0
            .run_mobile_plugin::<()>("clear", ())
            .map_err(|e| format!("{e}"))
    }

    /// Pre-initialize the session and request notification permissions.
    ///
    /// Optional — [`update_state`](Self::update_state) auto-initializes when needed.
    pub fn initialize(&self) -> Result<(), String> {
        self.0
            .run_mobile_plugin::<()>("initialize", ())
            .map_err(|e| format!("{e}"))
    }
}

/// Extension trait for accessing the media session from any Tauri manager.
#[cfg(mobile)]
pub trait MediaSessionExt<R: Runtime> {
    fn media_session(&self) -> &MediaSession<R>;
}

#[cfg(mobile)]
impl<R: Runtime, T: Manager<R>> MediaSessionExt<R> for T {
    fn media_session(&self) -> &MediaSession<R> {
        self.state::<MediaSession<R>>().inner()
    }
}

/// Initialize the media-session plugin.
///
/// On non-mobile platforms this registers a no-op plugin so that
/// cross-platform apps compile without `cfg` gates around the plugin call.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    register_commands(Builder::new("media-session"))
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            let handle = _api.register_android_plugin(PLUGIN_IDENTIFIER, "MediaSessionPlugin")?;

            #[cfg(target_os = "ios")]
            let handle = _api.register_ios_plugin(init_plugin_media_session)?;

            #[cfg(mobile)]
            _app.manage(MediaSession(handle));

            Ok(())
        })
        .build()
}

/// Register the JS-invocable commands.
///
/// Upstream 0.2.4 forgot to do this, which made the whole guest-js API
/// (including the action listener) unusable — fixed here.
fn register_commands<R: Runtime>(
    builder: tauri::plugin::Builder<R>,
) -> tauri::plugin::Builder<R> {
    // cfg 放在 let 上而不是函数上：rust-analyzer 对 build 脚本注入的
    // cfg(mobile)/cfg(desktop) 别名支持不完整，函数级门控会误报「找不到函数」
    #[cfg(mobile)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::initialize,
        commands::update_state,
        commands::update_timeline,
        commands::clear,
        commands::register_listener,
        commands::remove_listener
    ]);
    builder
}

#[cfg(mobile)]
mod commands;
