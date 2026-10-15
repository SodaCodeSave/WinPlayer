/**
 * 移动端判定：仅在 Tauri 运行于 Android / iOS 设备时为 true。
 * 通过 UA 检测（不引入 plugin-os 依赖）；桌面窗口无论多窄都不视为移动端。
 */
const userAgent = navigator.userAgent;
const isMobileDevice =
  /android|iphone|ipad|ipod|mobile/i.test(userAgent) ||
  (/macintosh/i.test(userAgent) && "ontouchend" in document); // iPadOS 伪装成 Mac 的桌面 UA

/** 平台在运行期不变，无需响应式 */
export function useIsMobile(): boolean {
  return isMobileDevice;
}
