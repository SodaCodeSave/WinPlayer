<script setup lang="ts">
import { useToasts } from "@/services/toast";

const items = useToasts();
</script>

<template>
  <div class="toast-host">
    <transition-group name="toast">
      <div v-for="item in items" :key="item.id" class="toast-item" :class="item.type">
        <span class="wp-icon toast-icon">
          {{ item.type === "error" ? "&#xE730;" : item.type === "success" ? "&#xE73E;" : "&#xE946;" }}
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
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-md) var(--space-xl);
  background: var(--wp-flyout);
  border: 1px solid var(--wp-control-stroke);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-flyout);
  font-size: 13px;
  max-width: 520px;
}

.toast-icon {
  font-size: 14px;
  flex: none;
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
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
