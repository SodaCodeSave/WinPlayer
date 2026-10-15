<script setup lang="ts">
/** 移动端二级页顶部返回栏（桌面端不渲染） */
import { useRouter } from "vue-router";
import { useIsMobile } from "@/composables/useIsMobile";

withDefaults(defineProps<{ title?: string }>(), { title: "" });

const router = useRouter();
const isMobile = useIsMobile();
</script>

<template>
  <div v-if="isMobile" class="mobile-page-header">
    <button class="back-button" title="返回" @click="router.back()">
      <span class="wp-icon">&#xE72B;</span>
    </button>
    <span v-if="title" class="header-title ellipsis">{{ title }}</span>
  </div>
</template>

<style scoped>
/* 反向吃掉 wp-page 的内边距，形成通顶吸顶栏 */
.mobile-page-header {
  position: sticky;
  top: 0;
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin: calc(-1 * var(--space-2xl)) calc(-1 * var(--space-3xl)) var(--space-xl);
  padding: var(--space-sm) var(--space-md);
  background: var(--wp-base);
  border-bottom: 1px solid var(--wp-divider);
  z-index: 10;
}

.back-button {
  width: 36px;
  height: 36px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--wp-text-primary);
  font-size: 14px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background var(--duration-fast) var(--ease-standard);
}

.back-button:active {
  background: var(--wp-card-active);
}

.header-title {
  flex: 1;
  min-width: 0;
  font-size: 15px;
  font-weight: 600;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
