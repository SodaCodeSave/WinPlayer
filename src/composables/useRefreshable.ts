import { onMounted, onUnmounted } from "vue";
import { setActiveRefresh } from "@/services/pullToRefresh";

/**
 * 为当前页面注册下拉刷新回调（仅移动端下拉手势会触发）。
 * 在组件的 setup 中调用即可，挂载时注册、卸载时清除。
 */
export function useRefreshable(handler: () => void | Promise<void>): void {
  onMounted(() => setActiveRefresh(handler));
  onUnmounted(() => setActiveRefresh(null));
}
