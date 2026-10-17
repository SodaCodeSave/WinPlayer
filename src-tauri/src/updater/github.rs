//! GitHub Release 查询：版本比较、安装包匹配与镜像加速。
//!
//! 仓库 release 的产物命名约定（见 .github/workflows/build.yml）：
//! `WinPlayer-{版本}-{平台}-{架构}[-类型].{扩展名}`
//! 例如 `WinPlayer-1.4.0-windows-x64-setup.exe`、`WinPlayer-1.4.0-android-universal.apk`。
//! 匹配时按「后缀」而非完整文件名比较（版本段随发布变化），并忽略大小写。

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::Duration;

use super::InstallerKind;

/// 上游仓库坐标
const REPO_OWNER: &str = "WinPlayer-dev";
const REPO_NAME: &str = "WinPlayer";

/// 需要镜像加速的 GitHub 域名前缀（与前端 services/githubMirror.ts 保持一致）
const GITHUB_URL_PREFIXES: [&str; 6] = [
    "https://github.com/",
    "https://raw.githubusercontent.com/",
    "https://codeload.github.com/",
    "https://api.github.com/",
    "https://gist.githubusercontent.com/",
    "https://objects.githubusercontent.com/",
];

/// API 请求超时（下载安装包不受此限制）
const API_TIMEOUT: Duration = Duration::from_secs(20);

/* ------------------------------ 数据结构 ------------------------------ */

/// GitHub release 条目（仅取所需字段）
#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

/// 可用的更新信息（`asset_url` 已应用镜像前缀）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// 去掉 `v` 前缀的版本号，如 `1.4.0`
    pub version: String,
    /// release 标签，如 `v1.4.0`
    pub tag: String,
    /// release 标题
    pub name: String,
    /// release 说明（Markdown 原文）
    pub notes: String,
    pub published_at: Option<String>,
    pub prerelease: bool,
    /// 当前平台将使用的安装包类型
    pub installer: InstallerKind,
    /// 安装包文件名
    pub asset_name: String,
    /// 安装包下载地址（已应用镜像前缀）
    pub asset_url: String,
    pub asset_size: u64,
    /// release 页面地址（手动下载入口）
    pub html_url: String,
}

/// 检查结果：两者同时为空表示已是最新版本
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    /// 有新版本且存在当前平台的安装包
    pub update: Option<UpdateInfo>,
    /// 存在更新的发布，但该发布没有当前平台可用的安装包
    pub unsupported_version: Option<String>,
}

/* ------------------------------ 平台与镜像 ------------------------------ */

/// 是否为「当前平台 + 架构」对应的 release 命名段
pub fn target_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "x86",
        "arm" => "arm",
        other => other,
    }
}

/// Android ABI（对应 `WinPlayer-x.y.z-android-{abi}.apk`）
fn android_abi() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "arm" => "arm",
        "x86" => "x86",
        "x86_64" => "x86_64",
        other => other,
    }
}

pub fn platform_name() -> &'static str {
    #[cfg(windows)]
    fn name() -> &'static str {
        "windows"
    }
    #[cfg(target_os = "macos")]
    fn name() -> &'static str {
        "macos"
    }
    #[cfg(target_os = "linux")]
    fn name() -> &'static str {
        "linux"
    }
    #[cfg(target_os = "android")]
    fn name() -> &'static str {
        "android"
    }
    #[cfg(target_os = "ios")]
    fn name() -> &'static str {
        "ios"
    }
    #[cfg(not(any(
        windows,
        target_os = "macos",
        target_os = "linux",
        target_os = "android",
        target_os = "ios"
    )))]
    fn name() -> &'static str {
        "unknown"
    }
    name()
}

/// 当前平台可用的安装包类型，按优先级排列（空表示不支持自动更新）
pub fn detect_installers() -> Vec<InstallerKind> {
    #[cfg(windows)]
    fn platform_installers() -> Vec<InstallerKind> {
        // Windows x64 同时提供 NSIS 与 MSI，优先 NSIS：静默安装不需要管理员权限
        if target_arch() == "x64" {
            vec![InstallerKind::WindowsNsis, InstallerKind::WindowsMsi]
        } else {
            vec![InstallerKind::WindowsNsis]
        }
    }
    #[cfg(target_os = "macos")]
    fn platform_installers() -> Vec<InstallerKind> {
        vec![InstallerKind::MacosDmg]
    }
    #[cfg(target_os = "linux")]
    fn platform_installers() -> Vec<InstallerKind> {
        use std::path::Path;

        // AppImage 运行时会设置 APPIMAGE 环境变量，据此判断当前安装形态
        if std::env::var_os("APPIMAGE").is_some() {
            vec![InstallerKind::LinuxAppimage]
        } else if Path::new("/usr/bin/apt-get").exists() || Path::new("/usr/bin/dpkg").exists() {
            vec![InstallerKind::LinuxDeb]
        } else if Path::new("/usr/bin/dnf").exists() || Path::new("/usr/bin/rpm").exists() {
            vec![InstallerKind::LinuxRpm]
        } else {
            // 兜底：AppImage 无需系统包管理器，任何发行版都能直接运行
            vec![InstallerKind::LinuxAppimage]
        }
    }
    #[cfg(target_os = "android")]
    fn platform_installers() -> Vec<InstallerKind> {
        vec![InstallerKind::AndroidApk]
    }
    #[cfg(not(any(
        windows,
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    fn platform_installers() -> Vec<InstallerKind> {
        Vec::new()
    }
    platform_installers()
}

/// GitHub 系地址拼接镜像前缀（如 `https://ghproxy.net/`），其余原样返回
pub fn apply_mirror(url: &str, mirror: &str) -> String {
    let mirror = mirror.trim().trim_end_matches('/');
    if mirror.is_empty() || !GITHUB_URL_PREFIXES.iter().any(|prefix| url.starts_with(prefix)) {
        return url.to_string();
    }
    format!("{mirror}/{url}")
}

/* ------------------------------ 版本比较 ------------------------------ */

/// 拆出「数字段」与「预发布段」（忽略构建元数据 `+...`）
fn split_version(version: &str) -> (Vec<u64>, Option<String>) {
    let version = version.trim().trim_start_matches(['v', 'V']);
    let version = version.split('+').next().unwrap_or(version);
    match version.split_once('-') {
        Some((core, pre)) => (parse_numbers(core), Some(pre.to_string())),
        None => (parse_numbers(version), None),
    }
}

fn parse_numbers(core: &str) -> Vec<u64> {
    core.split('.')
        .map(|part| part.trim().parse::<u64>().unwrap_or(0))
        .collect()
}

/// 语义化比较预发布段：数字标识 < 字母标识，逐段比较
fn compare_pre(a: &str, b: &str) -> Ordering {
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// 版本比较：相同为 Equal，a 更新为 Greater
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (a_nums, a_pre) = split_version(a);
    let (b_nums, b_pre) = split_version(b);
    for index in 0..a_nums.len().max(b_nums.len()) {
        let left = a_nums.get(index).copied().unwrap_or(0);
        let right = b_nums.get(index).copied().unwrap_or(0);
        if left != right {
            return left.cmp(&right);
        }
    }
    match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        // 正式版高于同号预发布版
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => compare_pre(&x, &y),
    }
}

/// 去掉标签的 `v` 前缀
pub fn normalize_version(tag: &str) -> String {
    tag.trim().trim_start_matches(['v', 'V']).to_string()
}

/* ------------------------------ 安装包匹配 ------------------------------ */

/// 当前平台上某种安装类型对应的文件名后缀（按优先级排列）
fn asset_suffixes(kind: InstallerKind) -> Vec<String> {
    let arch = target_arch();
    match kind {
        InstallerKind::WindowsNsis => vec![format!("-windows-{arch}-setup.exe")],
        InstallerKind::WindowsMsi => vec![format!("-windows-{arch}.msi")],
        InstallerKind::MacosDmg => vec![format!("-macos-{arch}.dmg")],
        InstallerKind::LinuxAppimage => vec![format!("-linux-{arch}.appimage")],
        InstallerKind::LinuxDeb => vec![format!("-linux-{arch}.deb")],
        InstallerKind::LinuxRpm => vec![format!("-linux-{arch}.rpm")],
        // 没有精确 ABI 的包时回退到通用包
        InstallerKind::AndroidApk => vec![
            format!("-android-{}.apk", android_abi()),
            "-android-universal.apk".to_string(),
        ],
    }
}

/// 在 release 产物中查找该安装类型对应的包
fn match_asset<'a>(assets: &'a [GhAsset], kind: InstallerKind) -> Option<&'a GhAsset> {
    for suffix in asset_suffixes(kind) {
        let suffix = suffix.to_ascii_lowercase();
        if let Some(asset) = assets.iter().find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.starts_with("winplayer-") && name.ends_with(&suffix)
        }) {
            return Some(asset);
        }
    }
    None
}

/* ------------------------------ 网络请求 ------------------------------ */

fn release_api_url(include_prerelease: bool, mirror: &str) -> String {
    let path = if include_prerelease {
        format!("/repos/{REPO_OWNER}/{REPO_NAME}/releases?per_page=20")
    } else {
        format!("/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest")
    };
    apply_mirror(&format!("https://api.github.com{path}"), mirror)
}

async fn fetch_releases(
    client: &reqwest::Client,
    include_prerelease: bool,
    mirror: &str,
) -> Result<Vec<GhRelease>, String> {
    let url = release_api_url(include_prerelease, mirror);
    let request = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    let response = tokio::time::timeout(API_TIMEOUT, request.send())
        .await
        .map_err(|_| "检查更新超时，请检查网络后重试".to_string())?
        .map_err(|e| format!("检查更新失败：{e}"))?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        // 仓库还没有任何 release
        return Ok(Vec::new());
    }
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let remaining = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        if remaining == "0" || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err("GitHub 接口请求过于频繁，请稍后重试（可在设置中配置 GitHub 镜像源）".into());
        }
        return Err(format!("检查更新失败：HTTP {}", status.as_u16()));
    }
    if !status.is_success() {
        return Err(format!("检查更新失败：HTTP {}", status.as_u16()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("读取更新信息失败：{e}"))?;

    // /releases/latest 返回单个对象，/releases 返回数组
    if include_prerelease {
        serde_json::from_str::<Vec<GhRelease>>(&text)
            .map_err(|e| format!("解析更新信息失败：{e}"))
    } else {
        serde_json::from_str::<GhRelease>(&text)
            .map(|release| vec![release])
            .map_err(|e| format!("解析更新信息失败：{e}"))
    }
}

/// 查询是否有比 `current_version` 更新的版本，并匹配当前平台的安装包
pub async fn fetch_update(
    client: &reqwest::Client,
    current_version: &str,
    include_prerelease: bool,
    mirror: &str,
) -> Result<CheckResult, String> {
    let installers = detect_installers();
    if installers.is_empty() {
        return Ok(CheckResult::default());
    }

    let releases = fetch_releases(client, include_prerelease, mirror).await?;
    let mut unsupported_version: Option<String> = None;

    for release in releases {
        if release.draft {
            continue;
        }
        if release.prerelease && !include_prerelease {
            continue;
        }
        let version = normalize_version(&release.tag_name);
        if compare_versions(&version, current_version) != Ordering::Greater {
            continue;
        }

        let matched = installers
            .iter()
            .find_map(|kind| match_asset(&release.assets, *kind).map(|asset| (*kind, asset)));

        match matched {
            Some((installer, asset)) => {
                let info = UpdateInfo {
                    version,
                    tag: release.tag_name.clone(),
                    name: release
                        .name
                        .clone()
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| release.tag_name.clone()),
                    notes: release.body.clone().unwrap_or_default(),
                    published_at: release.published_at.clone(),
                    prerelease: release.prerelease,
                    installer,
                    asset_name: asset.name.clone(),
                    asset_url: apply_mirror(&asset.browser_download_url, mirror),
                    asset_size: asset.size,
                    html_url: release.html_url.clone(),
                };
                return Ok(CheckResult {
                    update: Some(info),
                    unsupported_version: None,
                });
            }
            None => {
                // 记录「有新版本但没有本平台安装包」，供手动检查时提示
                if unsupported_version.is_none() {
                    unsupported_version = Some(version);
                }
            }
        }
    }

    Ok(CheckResult {
        update: None,
        unsupported_version,
    })
}

/// release 页面地址（手动下载入口），用于「前往 Release 页面」按钮
pub fn releases_page_url() -> String {
    format!("https://github.com/{REPO_OWNER}/{REPO_NAME}/releases")
}
