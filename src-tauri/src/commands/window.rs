//! 桌面窗口材质（Mica / Acrylic）：仅 Windows 桌面端有意义。
//!
//! 通过 window-vibrancy 调 DWM 接口给主窗口加系统级云母/亚克力材质；
//! 前端在材质生效时把主背景（--wp-base）换成半透明色让材质透出，
//! 并以 `<html data-material>` 同步实际生效的材质供 CSS 选择。

use window_vibrancy::{apply_acrylic, apply_mica, clear_acrylic, clear_mica};

/// 应用窗口材质（kind: "mica" | "acrylic" | "none"）。
///
/// Mica 仅 Windows 11 支持：请求 Mica 失败时自动回退 Acrylic，再失败回退
/// 不透明纯色背景。dark 传 None 时材质跟随系统亮暗偏好。
/// 返回实际生效的材质（"mica" / "acrylic" / "none"），前端据此切换 CSS。
#[tauri::command]
pub fn window_set_material(
    window: tauri::Window,
    kind: String,
    dark: Option<bool>,
) -> Result<String, String> {
    // 换材质前先清掉旧效果，避免 Mica（DWM 系统背板）与 Acrylic（合成属性）叠加
    let _ = clear_mica(&window);
    let _ = clear_acrylic(&window);

    let applied = match kind.as_str() {
        "mica" => apply_mica(&window, dark)
            .map(|_| "mica")
            // window-vibrancy 0.6 的 apply_acrylic 第二参数是 RGBA 色调；传 None 用默认底色
            .or_else(|_| apply_acrylic(&window, None::<(u8, u8, u8, u8)>).map(|_| "acrylic"))
            .unwrap_or("none"),
        "acrylic" => apply_acrylic(&window, None::<(u8, u8, u8, u8)>)
            .map(|_| "acrylic")
            .unwrap_or("none"),
        _ => "none",
    };
    Ok(applied.to_string())
}
