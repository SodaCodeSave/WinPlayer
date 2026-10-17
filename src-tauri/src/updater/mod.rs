//! 应用自动更新：检查 GitHub Release、下载安装包、按平台安装。
//!
//! - 检查：`updater_check` 查询 GitHub Releases，按 release 命名约定
//!   `WinPlayer-{版本}-{平台}-{架构}[-类型].{扩展名}` 匹配当前平台可用的安装包；
//! - 下载：`updater_download` 流式下载到应用缓存目录，期间通过 `update://progress`
//!   事件上报进度（约 200ms 一次），可用 `updater_download_cancel` 取消；
//! - 安装：`updater_install` 按平台拉起安装程序（Windows NSIS/MSI、macOS dmg、
//!   Linux AppImage/deb/rpm、Android APK），桌面端安装开始时自动退出应用。

pub mod github;
pub mod install;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub use github::CheckResult;

/// 下载进度事件名
const EVENT_PROGRESS: &str = "update://progress";
/// 进度事件的最小间隔，避免高频 IPC 拖慢下载
const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);

/* ------------------------------ 安装包类型 ------------------------------ */

/// 安装包类型：决定资产匹配后缀与安装方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallerKind {
    WindowsNsis,
    WindowsMsi,
    MacosDmg,
    LinuxAppimage,
    LinuxDeb,
    LinuxRpm,
    AndroidApk,
}

impl InstallerKind {
    /// 界面展示名称
    pub fn label(self) -> &'static str {
        match self {
            Self::WindowsNsis => "Windows 安装程序",
            Self::WindowsMsi => "Windows 安装包（MSI）",
            Self::MacosDmg => "macOS 磁盘映像",
            Self::LinuxAppimage => "Linux AppImage",
            Self::LinuxDeb => "Linux deb 包",
            Self::LinuxRpm => "Linux rpm 包",
            Self::AndroidApk => "Android APK",
        }
    }
}

/* ------------------------------ 数据结构 ------------------------------ */

/// 下载进度（节流后上报给前端）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    downloaded: u64,
    total: u64,
    /// 平均速度（字节/秒）
    speed: f64,
}

/// 下载结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    /// 安装包落盘路径
    path: String,
    size: u64,
}

/// 当前平台信息（关于页展示）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub platform: String,
    pub arch: String,
    /// 当前平台将使用的安装包类型；为空表示不支持自动更新
    pub installer: Option<InstallerKind>,
    pub installer_label: Option<String>,
    pub releases_url: String,
}

/* ------------------------------ 内部状态 ------------------------------ */

/// 下载取消标记：每次下载开始时重置
static CANCEL_DOWNLOAD: AtomicBool = AtomicBool::new(false);
/// 同一时刻只允许一个下载任务
static DOWNLOADING: AtomicBool = AtomicBool::new(false);

/// 共享 HTTP 客户端：只限制连接超时，安装包下载不受总时长限制
fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .user_agent("WinPlayer/0.1")
            .build()
            .expect("构建 HTTP 客户端失败")
    })
}

/// 更新包存放目录：应用缓存目录下的 updates/
fn updates_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_cache_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|e| format!("获取缓存目录失败：{e}"))?;
    let dir = base.join("updates");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建更新缓存目录失败：{e}"))?;
    Ok(dir)
}

/// 校验安装包文件名，只允许单层文件名（防止路径穿越）
fn safe_file_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    let valid = !name.is_empty()
        && name.len() <= 160
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'));
    if valid {
        Ok(name.to_string())
    } else {
        Err(format!("非法的安装包文件名：{name}"))
    }
}

/// 清理更新缓存目录中的旧安装包（保留 exclude）
fn clean_updates_dir(dir: &PathBuf, exclude: Option<&PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(exclude) = exclude {
            if path == *exclude {
                continue;
            }
        }
        let _ = std::fs::remove_file(path);
    }
}

/* ------------------------------ 命令 ------------------------------ */

/// 当前平台与可用的安装包类型
#[tauri::command]
pub fn updater_platform() -> PlatformInfo {
    let installer = github::detect_installers().first().copied();
    PlatformInfo {
        platform: github::platform_name().to_string(),
        arch: github::target_arch().to_string(),
        installer,
        installer_label: installer.map(|kind| kind.label().to_string()),
        releases_url: github::releases_page_url(),
    }
}

/// 检查更新；`current_version` 为当前应用版本（如 `1.5.0`）
#[tauri::command]
pub async fn updater_check(
    current_version: String,
    include_prerelease: bool,
    mirror: Option<String>,
) -> Result<CheckResult, String> {
    github::fetch_update(
        http_client(),
        current_version.trim(),
        include_prerelease,
        mirror.as_deref().unwrap_or(""),
    )
    .await
}

/// 下载安装包到更新缓存目录，期间通过 `update://progress` 上报进度
#[tauri::command]
pub async fn updater_download(
    app: AppHandle,
    url: String,
    file_name: String,
    expected_size: Option<u64>,
) -> Result<DownloadResult, String> {
    if url.trim().is_empty() {
        return Err("安装包地址为空".into());
    }
    if DOWNLOADING.swap(true, AtomicOrdering::SeqCst) {
        return Err("已有更新正在下载中".into());
    }
    // 无论成功失败都要释放下载标记
    let result = download_inner(&app, &url, &file_name, expected_size).await;
    DOWNLOADING.store(false, AtomicOrdering::SeqCst);
    result
}

async fn download_inner(
    app: &AppHandle,
    url: &str,
    file_name: &str,
    expected_size: Option<u64>,
) -> Result<DownloadResult, String> {
    use tokio::io::AsyncWriteExt;

    let file_name = safe_file_name(file_name)?;
    let dir = updates_dir(app)?;
    let target = dir.join(&file_name);
    // 先清掉上一次下载残留的安装包
    clean_updates_dir(&dir, Some(&target));

    CANCEL_DOWNLOAD.store(false, AtomicOrdering::SeqCst);

    let response = http_client()
        .get(url)
        // 要求服务端返回原始内容，保证 content-length 与实际字节数一致（进度准确）
        .header("Accept", "application/octet-stream")
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(|e| format!("下载失败：{e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("下载失败：HTTP {}", status.as_u16()));
    }

    let total = response
        .content_length()
        .or(expected_size)
        .unwrap_or(0);

    let temp = target.with_extension("download");
    let mut file = tokio::fs::File::create(&temp)
        .await
        .map_err(|e| format!("创建安装包文件失败：{e}"))?;

    let mut downloaded: u64 = 0;
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let mut response = response;

    loop {
        if CANCEL_DOWNLOAD.load(AtomicOrdering::SeqCst) {
            drop(file);
            let _ = tokio::fs::remove_file(&temp).await;
            return Err("已取消下载".into());
        }

        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(e) => {
                drop(file);
                let _ = tokio::fs::remove_file(&temp).await;
                return Err(format!("下载中断：{e}"));
            }
        };

        if let Err(e) = file.write_all(&chunk).await {
            drop(file);
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(format!("写入安装包失败：{e}"));
        }

        downloaded += chunk.len() as u64;
        if last_emit.elapsed() >= PROGRESS_INTERVAL || (total > 0 && downloaded >= total) {
            last_emit = Instant::now();
            emit_progress(app, downloaded, total, started);
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("写入安装包失败：{e}"))?;
    drop(file);

    if total > 0 && downloaded != total {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(format!("下载不完整（{downloaded}/{total} 字节），请重试"));
    }

    // 下载完成后再改名，避免未完成的文件被当作安装包
    tokio::fs::rename(&temp, &target)
        .await
        .map_err(|e| format!("保存安装包失败：{e}"))?;
    emit_progress(app, downloaded, total.max(downloaded), started);

    Ok(DownloadResult {
        path: target.to_string_lossy().to_string(),
        size: downloaded,
    })
}

fn emit_progress(app: &AppHandle, downloaded: u64, total: u64, started: Instant) {
    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    let _ = app.emit(
        EVENT_PROGRESS,
        ProgressPayload {
            downloaded,
            total,
            speed: downloaded as f64 / elapsed,
        },
    );
}

/// 取消正在进行的下载
#[tauri::command]
pub fn updater_download_cancel() {
    CANCEL_DOWNLOAD.store(true, AtomicOrdering::SeqCst);
}

/// 安装已下载的更新包；桌面端安装开始后自动退出应用
#[tauri::command]
pub async fn updater_install(
    app: AppHandle,
    path: String,
    installer: InstallerKind,
) -> Result<(), String> {
    let file = PathBuf::from(&path);
    if !file.is_file() {
        return Err(format!("安装包不存在：{path}"));
    }

    install::install(&app, installer, &file)?;

    // Android 由系统安装界面接管，应用继续运行；
    // 桌面端必须退出，安装程序才能替换正在使用的文件
    if installer != InstallerKind::AndroidApk {
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            handle.exit(0);
        });
    }

    Ok(())
}
