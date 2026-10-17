//! JS-invocable command wrappers around the mobile plugin.
//!
//! Each command forwards to the native side via `run_mobile_plugin`.
//! `register_listener` / `remove_listener` forward to the base plugin's
//! `registerListener` / `removeListener` so that `addPluginListener` from
//! `@tauri-apps/api/core` can wire up native events.

use serde_json::Value;
use tauri::{ipc::Channel, AppHandle, Runtime};

use crate::{MediaSessionExt, MediaState, TimelineUpdate};

#[tauri::command]
pub(crate) async fn initialize<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    app.media_session().initialize()
}

#[tauri::command]
pub(crate) async fn update_state<R: Runtime>(
    app: AppHandle<R>,
    state: MediaState,
) -> Result<(), String> {
    app.media_session().update_state(state)
}

#[tauri::command]
pub(crate) async fn update_timeline<R: Runtime>(
    app: AppHandle<R>,
    timeline: TimelineUpdate,
) -> Result<(), String> {
    app.media_session().update_timeline(timeline)
}

#[tauri::command]
pub(crate) async fn clear<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    app.media_session().clear()
}

#[tauri::command]
pub(crate) async fn register_listener<R: Runtime>(
    app: AppHandle<R>,
    event: String,
    handler: Channel<Value>,
) -> Result<(), String> {
    app.media_session()
        .forward("registerListener", event, handler)
}

#[tauri::command]
pub(crate) async fn remove_listener<R: Runtime>(
    app: AppHandle<R>,
    event: String,
    handler: Channel<Value>,
) -> Result<(), String> {
    app.media_session().forward("removeListener", event, handler)
}

trait Forward {
    fn forward(
        &self,
        method: &str,
        event: String,
        handler: Channel<Value>,
    ) -> Result<(), String>;
}

impl<R: Runtime> Forward for crate::MediaSession<R> {
    fn forward(
        &self,
        method: &str,
        event: String,
        handler: Channel<Value>,
    ) -> Result<(), String> {
        self.0
            .run_mobile_plugin(
                method,
                serde_json::json!({ "event": event, "handler": handler }),
            )
            .map_err(|e| format!("{e}"))
    }
}
