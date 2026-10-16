/** 轻量 Toast 通知 */

import { reactive } from "vue";

export interface ToastItem {
  id: number;
  message: string;
  type: "info" | "success" | "error";
}

const state = reactive<{ items: ToastItem[] }>({ items: [] });

let counter = 0;

export function toast(
  message: string,
  type: ToastItem["type"] = "info",
  duration?: number,
): void {
  counter += 1;
  const item: ToastItem = { id: counter, message, type };
  state.items.push(item);
  // 错误信息通常较长，停留更久以便完整阅读
  const actual = duration ?? (type === "error" ? 6500 : 3200);
  window.setTimeout(() => {
    const index = state.items.findIndex((t) => t.id === item.id);
    if (index >= 0) state.items.splice(index, 1);
  }, actual);
}

export function useToasts(): ToastItem[] {
  return state.items;
}
