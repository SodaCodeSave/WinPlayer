/** 内嵌右键菜单（替代 WebView 原生菜单） */

import { reactive } from "vue";

export interface ContextMenuItem {
  key: string;
  label: string;
  /** Segoe MDL2 图标实体，如 "&#xE8FD;" */
  icon?: string;
  danger?: boolean;
  disabled?: boolean;
  action: () => void | Promise<unknown>;
}

interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
}

export const contextMenuState = reactive<ContextMenuState>({
  visible: false,
  x: 0,
  y: 0,
  items: [],
});

/** 在鼠标位置打开内嵌右键菜单 */
export function openContextMenu(event: MouseEvent, items: ContextMenuItem[]): void {
  event.preventDefault();
  if (!items.length) return;
  contextMenuState.x = event.clientX;
  contextMenuState.y = event.clientY;
  contextMenuState.items = items;
  contextMenuState.visible = true;
}

export function closeContextMenu(): void {
  contextMenuState.visible = false;
}
