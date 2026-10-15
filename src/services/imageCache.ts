/**
 * 图片缓存 URL 改写。
 *
 * http(s) 图片地址统一改写为自定义协议 `cachedimg://`（Windows/WebView2 下
 * 实际访问形式为 http://cachedimg.localhost/），由 Rust 端处理：磁盘缓存命中
 * 直接读取，未命中时下载、压缩并落盘。因此改写后的 URL 始终显示缓存内容。
 * data: 等非网络地址原样透传。
 */
export function cachedImgUrl(url?: string | null): string | undefined {
  if (!url) return undefined;
  if (!/^https?:\/\//i.test(url)) return url;
  return `http://cachedimg.localhost/?src=${encodeURIComponent(url)}`;
}
