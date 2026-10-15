//! 系统媒体传输控件（SMTC）：把播放器快照同步给 Windows 媒体浮出窗 / 锁屏。
//!
//! 按钮事件（播放 / 暂停 / 上一首 / 下一首）由 Rust 侧主动 emit，见 `crate::smtc`。

use tauri::AppHandle;

use crate::smtc::SmtcSnapshot;

/// 前端每次把播放器的整体状态同步过来，Rust 侧 diff 后按需写入 WinRT 属性
#[tauri::command]
pub fn smtc_sync(app: AppHandle, snapshot: SmtcSnapshot) -> Result<(), String> {
    crate::smtc::sync(&app, &snapshot)
}
