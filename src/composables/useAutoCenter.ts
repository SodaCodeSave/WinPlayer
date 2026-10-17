/**
 * 列表自动回正：让「当前项」始终滚动居中。
 * - 面板打开或当前项变化时自动居中
 * - 用户手动滚动后暂停自动居中，停止滚动一段时间后自动回正
 */
import { nextTick, onBeforeUnmount, watch, type Ref } from "vue";

export interface AutoCenterOptions {
  /** 滚动容器 */
  container: Ref<HTMLElement | null>;
  /** 目标项选择器（相对容器查找，可命中嵌套元素） */
  itemSelector: string;
  /** 当前项下标；返回 -1 表示暂无目标 */
  getIndex: () => number;
  /** 面板是否处于激活（可见）状态；未激活时不滚动 */
  isActive: () => boolean;
  /** 手动滚动停止后自动回正的延迟（毫秒），默认 3000 */
  resumeDelay?: number;
}

/** 程序化滚动的判定窗口：窗口内的 scroll 事件视为自身触发，不计为手动滚动 */
const PROGRAMMATIC_WINDOW = 700;

export function useAutoCenter(options: AutoCenterOptions): void {
  const { container, itemSelector, getIndex, isActive, resumeDelay = 3000 } = options;

  /** 程序化滚动截止时间戳 */
  let programmaticUntil = 0;
  /** 用户手动滚动后是否暂停自动居中 */
  let paused = false;
  let resumeTimer: number | undefined;

  /** 把当前项滚动到容器垂直居中 */
  function center(behavior: ScrollBehavior): void {
    const el = container.value;
    const index = getIndex();
    if (!el || index < 0) return;
    const target = el.querySelectorAll<HTMLElement>(itemSelector)[index];
    if (!target) return;
    // 用相对容器的视口坐标计算，避免受 offsetParent 影响（容器未必是定位祖先）
    const elRect = el.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const centered =
      el.scrollTop + (targetRect.top - elRect.top) - (el.clientHeight - target.clientHeight) / 2;
    const max = Math.max(0, el.scrollHeight - el.clientHeight);
    const top = Math.min(Math.max(0, centered), max);
    if (Math.abs(top - el.scrollTop) < 1) return;
    programmaticUntil = Date.now() + PROGRAMMATIC_WINDOW;
    el.scrollTo({ top, behavior });
  }

  function onScroll(): void {
    if (Date.now() < programmaticUntil) return; // 自身滚动，忽略
    paused = true;
    window.clearTimeout(resumeTimer);
    resumeTimer = window.setTimeout(() => {
      paused = false;
      if (isActive()) void nextTick(() => center("smooth"));
    }, resumeDelay);
  }

  // 容器挂载 / 卸载时绑定滚动监听
  watch(
    container,
    (el, prev) => {
      prev?.removeEventListener("scroll", onScroll);
      el?.addEventListener("scroll", onScroll, { passive: true });
    },
    { immediate: true, flush: "post" },
  );

  watch(
    () => [isActive(), getIndex()] as const,
    async ([active], old) => {
      if (!active) return;
      const justOpened = !old?.[0];
      if (justOpened) {
        // 面板重新打开：清除暂停状态并清掉挂起的回正定时器
        paused = false;
        window.clearTimeout(resumeTimer);
      } else if (paused) {
        return; // 用户正在手动浏览，暂不打扰
      }
      await nextTick();
      center(justOpened ? "auto" : "smooth");
    },
    { flush: "post" },
  );

  onBeforeUnmount(() => {
    container.value?.removeEventListener("scroll", onScroll);
    window.clearTimeout(resumeTimer);
  });
}
