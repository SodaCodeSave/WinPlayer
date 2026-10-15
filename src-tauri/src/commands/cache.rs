//! 图片磁盘缓存：自定义协议 `cachedimg://`
//!
//! 前端把 http(s) 图片地址改写为 `http://cachedimg.localhost/?src=<encoded>`，
//! 本模块按「URL 哈希 → 磁盘文件」的策略处理请求：
//! - 命中：直接回源磁盘文件（图片始终走缓存，不重新请求网络）；
//! - 未命中：下载原图 → 压缩（最长边 512px、JPEG q75）→ 落盘 → 返回。
//!
//! 页面数据缓存（主页推荐/歌单详情）存于 SQLite settings 表，由前端 services/cache.ts 处理。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use tauri::Manager;
use tokio::sync::Semaphore;

/// 压缩后最长边（界面最大展示约 160px，2x 缩放下 512 足够清晰）
const MAX_DIMENSION: u32 = 512;
/// JPEG 压缩质量
const JPEG_QUALITY: u8 = 75;
/// 同一张图并发请求时的等待上限（对方正在下载）
const DEDUP_WAIT: Duration = Duration::from_secs(5);
/// 下载+压缩的并发上限：全尺寸 JPEG 解码很吃 CPU，不限制时长列表会打满所有核心
const MAX_CONCURRENT_FETCH: usize = 4;

fn pending() -> &'static Mutex<HashSet<String>> {
    static PENDING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashSet::new()))
}

fn fetch_limiter() -> &'static Semaphore {
    static LIMITER: OnceLock<Semaphore> = OnceLock::new();
    LIMITER.get_or_init(|| Semaphore::new(MAX_CONCURRENT_FETCH))
}

/// 共享 HTTP 客户端：复用连接池与 TLS 会话，避免每张图都重新握手
fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent("WinPlayer/0.1")
            .build()
            .expect("构建共享 HTTP 客户端失败")
    })
}

/// FNV-1a 64 位哈希，作为缓存文件名（64 位空间下碰撞概率可忽略）
fn hash_url(url: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in url.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn images_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {e}"))
        .map(|dir| dir.join("cache").join("images"))
}

fn image_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>, source_url: &str) -> Result<PathBuf, String> {
    Ok(images_dir(app)?.join(format!("{}.jpg", hash_url(source_url))))
}

fn not_found(message: &'static str) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(404)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(message.as_bytes().to_vec())
        .unwrap()
}

/// 处理一次图片请求：先查磁盘，未命中则下载并压缩落盘
async fn process_request<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let Ok(url) = tauri::Url::parse(&request.uri().to_string()) else {
        eprintln!("[cachedimg] bad request uri: {}", request.uri());
        return not_found("bad request");
    };
    let Some(source_url) = url
        .query_pairs()
        .find(|(key, _)| key == "src")
        .map(|(_, value)| value.into_owned())
    else {
        eprintln!("[cachedimg] missing src, uri={}", request.uri());
        return not_found("missing src");
    };
    // 只允许代理 http(s) 图片
    if !source_url.starts_with("http://") && !source_url.starts_with("https://") {
        eprintln!("[cachedimg] unsupported scheme: {source_url}");
        return not_found("unsupported scheme");
    }

    let Ok(path) = image_path(app, &source_url) else {
        eprintln!("[cachedimg] no cache dir for {source_url}");
        return not_found("no cache dir");
    };

    // 命中磁盘缓存：图片始终从缓存读取
    if let Ok(bytes) = tokio::fs::read(&path).await {
        return serve_jpeg(bytes);
    }

    // 去重：同一张图正在被其它请求下载时，轮询等待其落盘
    let mut acquired = false;
    {
        let mut set = pending().lock().expect("pending lock poisoned");
        if set.insert(hash_url(&source_url)) {
            acquired = true;
        }
    }
    if !acquired {
        let deadline = tokio::time::Instant::now() + DEDUP_WAIT;
        while tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Ok(bytes) = tokio::fs::read(&path).await {
                return serve_jpeg(bytes);
            }
        }
        eprintln!("[cachedimg] dedup wait timeout: {source_url}");
        return not_found("image fetch in progress");
    }

    let result = fetch_and_cache(&source_url, &path).await;
    pending()
        .lock()
        .expect("pending lock poisoned")
        .remove(&hash_url(&source_url));

    match result {
        Ok(bytes) => serve_jpeg(bytes),
        Err(e) => {
            eprintln!("[cachedimg] fetch failed for {source_url}: {e}");
            not_found("fetch failed")
        }
    }
}

fn serve_jpeg(bytes: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(200)
        .header("Content-Type", "image/jpeg")
        .header("Cache-Control", "max-age=31536000, immutable")
        .header("Access-Control-Allow-Origin", "*")
        .body(bytes)
        .unwrap()
}

/// 下载 → 解码 → 压缩 → 写入磁盘；解码失败时返回原始字节（不落盘）
async fn fetch_and_cache(source_url: &str, path: &PathBuf) -> Result<Vec<u8>, String> {
    // 限制并发：解码压缩是 CPU 大户，排队执行避免卡顿
    let _permit = fetch_limiter()
        .acquire()
        .await
        .map_err(|_| "并发限制器已关闭".to_string())?;

    let response = http_client()
        .get(source_url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取响应体失败: {e}"))?;

    // 压缩：解码后缩放到最长边 MAX_DIMENSION，再编码为 JPEG
    let compressed = tokio::task::spawn_blocking(move || -> Option<Vec<u8>> {
        let img = image::load_from_memory(&bytes).ok()?;
        let img = img.thumbnail(MAX_DIMENSION, MAX_DIMENSION);
        let mut buf = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY)
            .encode_image(&img)
            .ok()?;
        Some(buf)
    })
    .await
    .map_err(|e| format!("压缩任务失败: {e}"))?;

    match compressed {
        Some(jpeg) => {
            // 确保缓存目录存在（首次运行时尚无 cache/images），再写临时文件并改名
            if let Some(dir) = path.parent() {
                tokio::fs::create_dir_all(dir).await.map_err(|e| format!("创建缓存目录失败: {e}"))?;
            }
            let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
            tokio::fs::write(&tmp, &jpeg).await.map_err(|e| format!("写缓存失败: {e}"))?;
            if tokio::fs::rename(&tmp, path).await.is_err() {
                let _ = tokio::fs::remove_file(&tmp).await;
            }
            Ok(jpeg)
        }
        // 内容不是可解码图片（如 SVG）：直接透传，不缓存
        None => Err("图片解码失败".to_string()),
    }
}

/// 注册 cachedimg 自定义协议（Windows 下通过 http://cachedimg.localhost/ 访问）
pub fn register<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.register_asynchronous_uri_scheme_protocol("cachedimg", |ctx, request, responder| {
        let app = ctx.app_handle().clone();
        tauri::async_runtime::spawn(async move {
            responder.respond(process_request(&app, request).await);
        });
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    image_count: u64,
    image_bytes: u64,
}

/// 统计图片缓存目录（页面数据缓存在 SQLite，由前端统计）
fn scan_images(dir: &PathBuf) -> CacheStats {
    let mut stats = CacheStats { image_count: 0, image_bytes: 0 };
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    stats.image_count += 1;
                    stats.image_bytes += meta.len();
                }
            }
        }
    }
    stats
}

#[tauri::command]
pub async fn cache_stats(app: tauri::AppHandle) -> Result<CacheStats, String> {
    let dir = images_dir(&app)?;
    Ok(scan_images(&dir))
}

/// 清空图片缓存，返回清理前的占用（页面数据缓存由前端另行删除）
#[tauri::command]
pub async fn cache_clear(app: tauri::AppHandle) -> Result<CacheStats, String> {
    let dir = images_dir(&app)?;
    let stats = scan_images(&dir);
    if dir.exists() {
        tokio::fs::remove_dir_all(&dir)
            .await
            .map_err(|e| format!("删除缓存目录失败: {e}"))?;
    }
    Ok(stats)
}
