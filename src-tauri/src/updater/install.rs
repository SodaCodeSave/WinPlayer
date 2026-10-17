//! 按平台安装已下载的更新包。
//!
//! 各平台的处理方式：
//! - Windows：NSIS 安装器 `setup.exe /S /R`（静默安装 + 安装后自动重启应用），
//!   无 NSIS 时退回 MSI 的 `msiexec /i ... /qb`；
//! - macOS：后台脚本等待应用退出后挂载 dmg、用 ditto 替换 `.app` 并重新打开；
//! - Linux：AppImage 就地替换并重启；deb/rpm 经 pkexec 调包管理器安装后重启；
//! - Android：交给原生安装器（系统弹窗确认），应用继续运行。
//!
//! 桌面端的所有安装动作都不阻塞应用：拉起安装程序/脚本后由调用方退出应用，
//! 让安装程序能够替换正在使用的文件。

use std::path::Path;

// 仅桌面端的安装实现会拉起子进程（Windows 安装器 / macOS、Linux 脚本），Android 用不到
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
use std::process::{Command, Stdio};

use tauri::{AppHandle, Runtime};

use super::InstallerKind;

/// 启动安装流程（返回后调用方负责按需退出应用）
pub fn install<R: Runtime>(
    app: &AppHandle<R>,
    kind: InstallerKind,
    path: &Path,
) -> Result<(), String> {
    // 各平台的实现用不到全部参数，这里统一标记为已使用，避免未使用告警
    let _ = app;
    let _ = kind;
    let _ = path;

    match kind {
        #[cfg(windows)]
        InstallerKind::WindowsNsis | InstallerKind::WindowsMsi => install_windows(kind, path),
        #[cfg(target_os = "macos")]
        InstallerKind::MacosDmg => install_macos(path),
        #[cfg(target_os = "linux")]
        InstallerKind::LinuxAppimage => install_appimage(path),
        #[cfg(target_os = "linux")]
        InstallerKind::LinuxDeb | InstallerKind::LinuxRpm => install_linux_package(path),
        #[cfg(target_os = "android")]
        InstallerKind::AndroidApk => install_android(app, path),
        other => Err(format!(
            "当前平台不支持安装「{}」格式的更新包，请前往 Release 页面手动下载",
            other.label()
        )),
    }
}

/* ------------------------------ Windows ------------------------------ */

#[cfg(windows)]
fn install_windows(kind: InstallerKind, path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    /// 静默启动子进程时不弹出控制台窗口
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let path_str = path.to_string_lossy().to_string();
    let mut command = if kind == InstallerKind::WindowsMsi {
        // MSI 无法自行重启应用，安装完成后由用户手动打开（NSIS 是首选，走到这里的概率很低）
        let mut command = Command::new("msiexec");
        command.args(["/i", path_str.as_str(), "/qb"]);
        command
    } else {
        // Tauri 的 NSIS 安装器支持 /S（静默）与 /R（安装后重启应用）
        let mut command = Command::new(path);
        command.args(["/S", "/R"]);
        command
    };

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("启动安装程序失败：{e}"))?;
    Ok(())
}

/* ------------------------------ macOS ------------------------------ */

#[cfg(target_os = "macos")]
const MACOS_INSTALL_SCRIPT: &str = r#"#!/bin/sh
# WinPlayer 更新脚本：等待主进程退出 -> 挂载 dmg -> 替换 .app -> 重新启动
PID="$1"
BUNDLE="$2"
DMG="$3"

while kill -0 "$PID" 2>/dev/null; do
  sleep 0.5
done

MOUNT="$(mktemp -d /tmp/winplayer-update.XXXXXX)"
if ! hdiutil attach "$DMG" -mountpoint "$MOUNT" -nobrowse -quiet; then
  open "$DMG"
  exit 0
fi

SRC="$(ls -d "$MOUNT"/*.app 2>/dev/null | head -n 1)"
if [ -z "$SRC" ]; then
  hdiutil detach "$MOUNT" -quiet
  open "$DMG"
  exit 0
fi

if [ -w "$(dirname "$BUNDLE")" ]; then
  rm -rf "$BUNDLE.new"
  if ditto "$SRC" "$BUNDLE.new"; then
    rm -rf "$BUNDLE"
    mv "$BUNDLE.new" "$BUNDLE"
    xattr -dr com.apple.quarantine "$BUNDLE" 2>/dev/null
    hdiutil detach "$MOUNT" -quiet
    rm -f "$DMG"
    open "$BUNDLE"
    exit 0
  fi
fi

hdiutil detach "$MOUNT" -quiet
# 无法就地替换（安装目录不可写）时打开 dmg 由用户手动安装
open "$DMG"
"#;

#[cfg(target_os = "macos")]
fn install_macos(path: &Path) -> Result<(), String> {
    let bundle = current_app_bundle()?;
    spawn_script(
        "macos",
        MACOS_INSTALL_SCRIPT,
        &[
            std::process::id().to_string(),
            bundle,
            path.to_string_lossy().to_string(),
        ],
    )
}

/// 从可执行文件路径向上找到 `.app` 包目录
#[cfg(target_os = "macos")]
fn current_app_bundle() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("获取应用路径失败：{e}"))?;
    for ancestor in exe.ancestors() {
        if ancestor.extension().and_then(|ext| ext.to_str()) == Some("app") {
            return Ok(ancestor.to_string_lossy().to_string());
        }
    }
    Err("无法定位应用包（.app），请前往 Release 页面手动下载".into())
}

/* ------------------------------ Linux ------------------------------ */

#[cfg(target_os = "linux")]
const APPIMAGE_INSTALL_SCRIPT: &str = r#"#!/bin/sh
# WinPlayer 更新脚本：等待主进程退出 -> 覆盖 AppImage 文件 -> 重新启动
PID="$1"
CURRENT="$2"
NEW="$3"

while kill -0 "$PID" 2>/dev/null; do
  sleep 0.5
done

chmod +x "$NEW" 2>/dev/null

if [ -w "$(dirname "$CURRENT")" ]; then
  if mv -f "$NEW" "$CURRENT"; then
    exec "$CURRENT"
  fi
fi

# 目录不可写：把新包放进下载目录并打开所在文件夹，由用户手动替换
DEST="$HOME/Downloads"
if [ ! -d "$DEST" ]; then
  DEST="$HOME"
fi
mv -f "$NEW" "$DEST/$(basename "$NEW")" 2>/dev/null
xdg-open "$DEST" >/dev/null 2>&1
"#;

#[cfg(target_os = "linux")]
const PACKAGE_INSTALL_SCRIPT: &str = r#"#!/bin/sh
# WinPlayer 更新脚本：等待主进程退出 -> 调包管理器安装 -> 重新启动
PID="$1"
PKG="$2"
EXE="$3"

while kill -0 "$PID" 2>/dev/null; do
  sleep 0.5
done

if command -v apt-get >/dev/null 2>&1; then
  pkexec apt-get install -y "$PKG"
elif command -v dpkg >/dev/null 2>&1; then
  pkexec dpkg -i "$PKG"
elif command -v dnf >/dev/null 2>&1; then
  pkexec dnf install -y "$PKG"
elif command -v rpm >/dev/null 2>&1; then
  pkexec rpm -Uvh --replacepkgs "$PKG"
fi

if [ -n "$EXE" ] && [ -x "$EXE" ]; then
  nohup "$EXE" >/dev/null 2>&1 &
fi
"#;

#[cfg(target_os = "linux")]
fn install_appimage(path: &Path) -> Result<(), String> {
    let current = std::env::var("APPIMAGE")
        .map_err(|_| "未检测到 AppImage 运行环境，请前往 Release 页面手动下载".to_string())?;
    spawn_script(
        "appimage",
        APPIMAGE_INSTALL_SCRIPT,
        &[
            std::process::id().to_string(),
            current,
            path.to_string_lossy().to_string(),
        ],
    )
}

#[cfg(target_os = "linux")]
fn install_linux_package(path: &Path) -> Result<(), String> {
    if find_executable("pkexec").is_none() {
        return Err(format!(
            "未找到 pkexec（polkit），无法自动安装；安装包已下载到 {}，请手动安装",
            path.display()
        ));
    }
    let exe = std::env::current_exe()
        .map(|exe| exe.to_string_lossy().to_string())
        .unwrap_or_default();
    spawn_script(
        "package",
        PACKAGE_INSTALL_SCRIPT,
        &[
            std::process::id().to_string(),
            path.to_string_lossy().to_string(),
            exe,
        ],
    )
}

/// 在常见目录中查找可执行文件（不依赖登录 shell 的 PATH）
#[cfg(target_os = "linux")]
fn find_executable(name: &str) -> Option<()> {
    ["/usr/bin", "/bin", "/usr/local/bin", "/usr/sbin", "/sbin"]
        .iter()
        .find(|dir| Path::new(dir).join(name).is_file())
        .map(|_| ())
}

/// 把脚本写入临时目录并脱离当前进程启动（父进程退出后继续执行）
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn spawn_script(name: &str, script: &str, args: &[String]) -> Result<(), String> {
    let dir = std::env::temp_dir().join("winplayer-update");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建临时目录失败：{e}"))?;
    let script_path = dir.join(format!("{name}-{}.sh", std::process::id()));
    std::fs::write(&script_path, script).map_err(|e| format!("写入更新脚本失败：{e}"))?;

    Command::new("/bin/sh")
        .arg(&script_path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("启动更新脚本失败：{e}"))?;
    Ok(())
}

/* ------------------------------ Android ------------------------------ */

#[cfg(target_os = "android")]
fn install_android<R: Runtime>(app: &AppHandle<R>, path: &Path) -> Result<(), String> {
    use tauri_plugin_apk_installer::ApkInstallerExt;

    app.apk_installer()
        .install_apk(path.to_string_lossy().to_string())
        .map_err(|e| format!("调起安装器失败：{e}"))
}
