import type { Router } from "vue-router";

/**
 * 移动端「返回恢复滚动位置」。
 *
 * 应用的主滚动容器是路由组件的根节点（.app-content 的唯一子元素），而不是
 * window，所以 vue-router 内置的 scrollBehavior 不适用，这里手动记录 / 恢复。
 *
 * 位置按 route.fullPath 记录，采取「恢复一次即消费」策略：
 * - 返回上一页时恢复其离开时的位置
 * - 恢复后清除记录，之后若再从别的入口前进到同一路由，则从顶部开始
 */

/** 内容为异步加载时，恢复滚动需要重试的帧数上限（约 1 秒 @60fps） */
const MAX_RESTORE_FRAMES = 60;
/** 等待「旧页面离场、新页面挂载」的帧数上限（out-in 过渡） */
const MAX_WAIT_FRAMES = 60;

export function installScrollMemory(router: Router): void {
  const positions = new Map<string, number>();

  /** 当前页面的滚动容器 = .app-content 的第一个元素子节点 */
  function getScroller(): HTMLElement | null {
    const content = document.querySelector<HTMLElement>(".app-content");
    if (!content) return null;
    for (const child of Array.from(content.children)) {
      if (child instanceof HTMLElement) return child;
    }
    return null;
  }

  router.beforeEach((_to, from) => {
    if (!from.name) return; // 初始加载：还没有可记录的页面
    const el = getScroller();
    if (el) positions.set(from.fullPath, el.scrollTop);
  });

  router.afterEach((to) => {
    const target = positions.get(to.fullPath);
    if (target === undefined) return;
    // 消费掉记录，避免下次主动前进到同一路由时莫名滚动
    positions.delete(to.fullPath);

    // out-in 过渡：新组件要等旧组件离场后才挂载，先等它出现
    const oldEl = getScroller();
    let waited = 0;
    const waitNew = () => {
      const el = getScroller();
      if (el && el !== oldEl) {
        applyScroll(el, target);
        return;
      }
      if (waited++ < MAX_WAIT_FRAMES) requestAnimationFrame(waitNew);
    };
    requestAnimationFrame(waitNew);
  });
}

/** 持续重试直到滚动落位（内容异步加载时高度会先小后大）或超时 */
function applyScroll(el: HTMLElement, target: number): void {
  let tries = 0;
  const tick = () => {
    if (el.scrollTop !== target && tries++ < MAX_RESTORE_FRAMES) {
      el.scrollTop = target;
      requestAnimationFrame(tick);
    }
  };
  el.scrollTop = target;
  requestAnimationFrame(tick);
}
