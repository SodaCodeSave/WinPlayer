<script setup lang="ts">
import { useToasts } from "@/services/toast";

const items = useToasts();
</script>

<template>
  <div class="toast-host">
    <transition-group name="toast">
      <div v-for="item in items" :key="item.id" class="toast-item" :class="item.type">
        <span class="wp-icon toast-icon">
          {{ item.type === "error" ? "&#xF3F2;" : item.type === "success" ? "&#xF299;" : "&#xF4A4;" }}
        </span>
        <span class="toast-message">{{ item.message }}</span>
      </div>
    </transition-group>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  bottom: calc(var(--playerbar-height) + var(--space-xl));
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-md);
  z-index: 100;
  pointer-events: none;
}

.toast-item {
  display: flex;
  align-items: flex-start;
  gap: var(--space-md);
  padding: var(--space-md) var(--space-xl);
  background: var(--wp-flyout);
  border: 1px solid var(--wp-control-stroke);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-flyout);
  font-size: 13px;
  max-width: 560px;
}

.toast-icon {
  font-size: 14px;
  flex: none;
  margin-top: 1px;
}

.toast-item.error .toast-icon {
  color: var(--wp-danger);
}

.toast-item.success .toast-icon {
  color: var(--wp-success);
}

.toast-item.info .toast-icon {
  color: var(--wp-accent);
}

.toast-message {
  /* 长报错信息需完整可读：允许多行换行而非省略号截断 */
  line-height: 1.5;
  word-break: break-word;
}

.toast-enter-active,
.toast-leave-active {
  transition:
    transform var(--duration-normal) var(--ease-decelerate),
    opacity var(--duration-normal) var(--ease-decelerate);
}

.toast-enter-from,
.toast-leave-to {
  transform: translateY(12px);
  opacity: 0;
}
</style>
