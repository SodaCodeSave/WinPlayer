<script setup lang="ts">
/**
 * 内嵌右键菜单宿主（全局唯一实例，挂在 App.vue）
 * - 在元素上调用 openContextMenu(event, items) 即可在鼠标位置弹出
 * - 全局拦截右键：非可编辑区域一律屏蔽系统原生菜单
 */
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import {
  closeContextMenu,
  contextMenuState,
  type ContextMenuItem,
} from "@/services/contextMenu";

const menuRef = ref<HTMLElement | null>(null);

/** 弹出后测量尺寸，避免超出视口边缘 */
watch(
  () => contextMenuState.visible,
  async (visible) => {
    if (!visible) return;
    await nextTick();
    const el = menuRef.value;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    if (contextMenuState.x + rect.width > window.innerWidth - 8) {
      contextMenuState.x = Math.max(8, window.innerWidth - rect.width - 8);
    }
    if (contextMenuState.y + rect.height > window.innerHeight - 8) {
      contextMenuState.y = Math.max(8, window.innerHeight - rect.height - 8);
    }
  },
);

function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return target.closest("input, textarea, [contenteditable]") !== null;
}

async function activate(item: ContextMenuItem) {
  if (item.disabled) return;
  closeContextMenu();
  await item.action();
}

function onWindowPointerDown(event: PointerEvent) {
  if (!contextMenuState.visible) return;
  if (menuRef.value && event.target instanceof Node && menuRef.value.contains(event.target)) return;
  closeContextMenu();
}

function onWindowKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && contextMenuState.visible) closeContextMenu();
}

/** 非可编辑区域统一屏蔽系统右键菜单；未被业务处理过的位置顺带关闭已弹出的菜单 */
function onWindowContextMenu(event: MouseEvent) {
  if (isEditable(event.target)) return;
  if (!event.defaultPrevented) {
    event.preventDefault();
    if (contextMenuState.visible) closeContextMenu();
  }
}

function onWindowBlur() {
  if (contextMenuState.visible) closeContextMenu();
}

onMounted(() => {
  window.addEventListener("pointerdown", onWindowPointerDown, true);
  window.addEventListener("keydown", onWindowKeydown);
  window.addEventListener("contextmenu", onWindowContextMenu);
  window.addEventListener("blur", onWindowBlur);
});

onUnmounted(() => {
  window.removeEventListener("pointerdown", onWindowPointerDown, true);
  window.removeEventListener("keydown", onWindowKeydown);
  window.removeEventListener("contextmenu", onWindowContextMenu);
  window.removeEventListener("blur", onWindowBlur);
});
</script>

<template>
  <teleport to="body">
    <transition name="ctx-menu">
      <div
        v-if="contextMenuState.visible"
        ref="menuRef"
        class="ctx-menu"
        :style="{ left: `${contextMenuState.x}px`, top: `${contextMenuState.y}px` }"
        @contextmenu.prevent.stop
      >
        <button
          v-for="item in contextMenuState.items"
          :key="item.key"
          class="ctx-item"
          :class="{ danger: item.danger }"
          :disabled="item.disabled"
          @click="activate(item)"
        >
          <span
            v-if="item.icon"
            class="wp-icon ctx-icon"
            :class="{ filled: item.filled }"
            v-html="item.icon"
          ></span>
          <span class="ctx-label">{{ item.label }}</span>
        </button>
      </div>
    </transition>
  </teleport>
</template>

<style scoped>
.ctx-menu {
  position: fixed;
  z-index: 1000;
  min-width: 180px;
  padding: 4px;
  background: var(--wp-flyout);
  border: 1px solid var(--wp-control-stroke);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-flyout);
  display: flex;
  flex-direction: column;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  height: 34px;
  padding: 0 var(--space-lg);
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  font: inherit;
  font-size: 13px;
  color: var(--wp-text-primary);
  cursor: pointer;
  text-align: left;
  transition: background var(--duration-fast) var(--ease-standard);
}

.ctx-item:hover:not(:disabled) {
  background: var(--wp-card-hover);
}

.ctx-item:disabled {
  opacity: 0.5;
  cursor: default;
}

.ctx-item.danger {
  color: var(--wp-danger);
}

.ctx-icon {
  font-size: 14px;
  color: var(--wp-text-secondary);
  flex: none;
}

.ctx-item.danger .ctx-icon {
  color: var(--wp-danger);
}

.ctx-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 弹出动画（WinUI 飞入近似） */
.ctx-menu-enter-active,
.ctx-menu-leave-active {
  transition:
    transform var(--duration-fast) var(--ease-decelerate),
    opacity var(--duration-fast) var(--ease-decelerate);
}

.ctx-menu-enter-from,
.ctx-menu-leave-to {
  transform: scale(0.96);
  opacity: 0;
}
</style>
