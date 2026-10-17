//! Android APK 安装插件（桌面端为 no-op）。
//!
//! 下载好的 APK 位于应用私有目录。Android 7 之后直接在 Intent 里传
//! `file://` 会抛 `FileUriExposedException`，因此原生侧先用 FileProvider
//! 把 APK 转成 `content://` URI，再带上读权限启动系统软件包安装器。
//!
//! 本插件不注册任何前端命令：应用侧 Rust 代码通过扩展 trait 调用原生能力。

use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

#[cfg(mobile)]
use tauri::Manager;

#[cfg(mobile)]
use serde::{Deserialize, Serialize};

/// 与 ApkInstallerPlugin.kt 中的 @TauriPlugin 对应的标识符
#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.tauri.apkinstaller";

/// 传给原生侧的参数
#[cfg(mobile)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallApkArgs {
    /// APK 的绝对路径
    pub path: String,
}

/// 原生安装器的句柄
#[cfg(mobile)]
pub struct ApkInstaller<R: Runtime>(tauri::plugin::PluginHandle<R>);

#[cfg(mobile)]
impl<R: Runtime> ApkInstaller<R> {
    /// 调起系统安装器安装指定 APK。
    ///
    /// 返回 `Ok(())` 只代表安装界面已成功调起，用户仍需在系统弹窗中确认。
    pub fn install_apk(&self, path: String) -> Result<(), String> {
        self.0
            .run_mobile_plugin::<()>("installApk", InstallApkArgs { path })
            .map_err(|e| format!("{e}"))
    }
}

/// 从任意 Tauri Manager 取到原生安装器
#[cfg(mobile)]
pub trait ApkInstallerExt<R: Runtime> {
    fn apk_installer(&self) -> &ApkInstaller<R>;
}

#[cfg(mobile)]
impl<R: Runtime, T: Manager<R>> ApkInstallerExt<R> for T {
    fn apk_installer(&self) -> &ApkInstaller<R> {
        self.state::<ApkInstaller<R>>().inner()
    }
}

/// 初始化插件；桌面端注册一个空插件，保证跨平台编译无需 cfg 门控。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("apk-installer")
        .setup(|_app, _api| {
            // 仅 Android 注册原生实现（其他平台没有对应的 Kotlin 类）
            #[cfg(target_os = "android")]
            {
                let handle =
                    _api.register_android_plugin(PLUGIN_IDENTIFIER, "ApkInstallerPlugin")?;
                _app.manage(ApkInstaller(handle));
            }

            Ok(())
        })
        .build()
}
