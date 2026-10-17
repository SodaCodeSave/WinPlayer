/**
 * 图片缓存 URL 改写。
 *
 * http(s) 图片地址统一改写为自定义协议 `cachedimg://`（Windows/WebView2 下
 * 实际访问形式为 http://cachedimg.localhost/），由 Rust 端处理：磁盘缓存命中
 * 直接读取，未命中时下载、压缩并落盘。因此改写后的 URL 始终显示缓存内容。
 * data: 等非网络地址原样透传。
 *
 * 设置中「缓存项」关闭图片缓存时，不再改写，直接返回原图地址直连加载。
 */

import { useSettingsStore } from "@/stores/settings";

/** 当前策略下是否允许图片磁盘缓存 */
export function isImageCacheEnabled(): boolean {
  const policy = useSettingsStore().cachePolicy;
  return policy === "both" || policy === "images";
}

export function cachedImgUrl(url?: string | null): string | undefined {
  if (!url) return undefined;
  if (!/^https?:\/\//i.test(url)) return url;
  if (!isImageCacheEnabled()) return url;
  return `http://cachedimg.localhost/?src=${encodeURIComponent(url)}`;
}
