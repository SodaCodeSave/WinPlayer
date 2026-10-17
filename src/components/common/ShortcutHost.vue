<script setup lang="ts">
/**
 * 全局快捷键宿主（挂在 App.vue，全局唯一实例）
 * - 在 window 上挂载 keydown 监听，命中可配置快捷键或内置空格键时驱动播放器
 * - 仅在窗口处于前台（渲染进程能收到键盘事件）时生效，不注册系统级全局热键
 * - 移动端无物理键盘，不挂载监听
 */
import { onMounted, onUnmounted } from "vue";
import { useIsMobile } from "@/composables/useIsMobile";
import { useShortcutsStore } from "@/stores/shortcuts";

const shortcuts = useShortcutsStore();
const isMobile = useIsMobile();

function onKeydown(event: KeyboardEvent): void {
  shortcuts.handleKeydown(event);
}

onMounted(() => {
  if (!isMobile) window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <span hidden aria-hidden="true" />
</template>
