/** 轻量 Toast 通知 */

import { reactive } from "vue";

export interface ToastItem {
  id: number;
  message: string;
  type: "info" | "success" | "error";
}

const state = reactive<{ items: ToastItem[] }>({ items: [] });

let counter = 0;

export function toast(message: string, type: ToastItem["type"] = "info", duration = 3200): void {
  counter += 1;
  const item: ToastItem = { id: counter, message, type };
  state.items.push(item);
  window.setTimeout(() => {
    const index = state.items.findIndex((t) => t.id === item.id);
    if (index >= 0) state.items.splice(index, 1);
  }, duration);
}

export function useToasts(): ToastItem[] {
  return state.items;
}
