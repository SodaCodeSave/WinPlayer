<script setup lang="ts">
import { usePlayerStore } from "@/stores/player";
import { formatDuration, trackArtist, trackTitle } from "@/types/player";

const player = usePlayerStore();
</script>

<template>
  <transition name="panel">
    <div v-if="player.showQueue" class="queue-panel" @pointerdown.stop>
      <div class="panel-header">
        <span class="panel-title">播放队列</span>
        <span class="panel-count">{{ player.queue.length }} 首</span>
        <button v-if="player.queue.length" class="panel-action" @click="player.clearQueue">清空</button>
      </div>

      <div class="panel-body">
        <div v-if="!player.queue.length" class="panel-empty">队列为空，去发现音乐吧</div>
        <ul v-else class="queue-list">
          <li
            v-for="(item, index) in player.queue"
            :key="item.uid"
            class="queue-item"
            :class="{ current: index === player.currentIndex }"
            draggable="true"
            @dragstart.stop
            @dblclick="player.playAt(index)"
          >
            <div class="queue-index">
              <span v-if="index === player.currentIndex" class="wp-icon playing-indicator">&#xE769;</span>
              <span v-else>{{ index + 1 }}</span>
            </div>
            <div class="queue-meta">
              <div class="queue-title ellipsis">{{ trackTitle(item.track) }}</div>
              <div class="queue-artist ellipsis">{{ trackArtist(item.track) }}</div>
            </div>
            <span class="queue-duration">{{ formatDuration(item.track.duration) }}</span>
            <button class="queue-remove" title="移除" @click.stop="player.removeFromQueue(item.uid)">
              <span class="wp-icon">&#xE711;</span>
            </button>
          </li>
        </ul>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.queue-panel {
  position: absolute;
  right: var(--space-xl);
  bottom: calc(100% + var(--space-md));
  width: 380px;
  max-height: 480px;
  display: flex;
  flex-direction: column;
  background: var(--wp-flyout);
  border: 1px solid var(--wp-control-stroke);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-flyout);
  z-index: 50;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-lg) var(--space-xl);
  border-bottom: 1px solid var(--wp-divider);
}

.panel-title {
  font-size: 14px;
  font-weight: 600;
}

.panel-count {
  font-size: 12px;
  color: var(--wp-text-tertiary);
  flex: 1;
}

.panel-action {
  border: none;
  background: transparent;
  color: var(--wp-accent);
  font-size: 12px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
}

.panel-action:hover {
  background: var(--wp-card-hover);
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: var(--space-sm);
}

.panel-empty {
  padding: var(--space-3xl) var(--space-xl);
  text-align: center;
  color: var(--wp-text-tertiary);
  font-size: 13px;
}

.queue-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.queue-item {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  height: 48px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-sm);
  cursor: default;
  transition: background var(--duration-fast) var(--ease-standard);
}

.queue-item:hover {
  background: var(--wp-card-hover);
}

.queue-item.current .queue-title {
  color: var(--wp-accent);
}

.queue-index {
  width: 28px;
  flex: none;
  text-align: center;
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.playing-indicator {
  color: var(--wp-accent);
  font-size: 12px;
}

.queue-meta {
  flex: 1;
  min-width: 0;
}

.queue-title {
  font-size: 13px;
}

.queue-artist {
  font-size: 11px;
  color: var(--wp-text-secondary);
  margin-top: 1px;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.queue-duration {
  font-size: 11px;
  color: var(--wp-text-tertiary);
  font-variant-numeric: tabular-nums;
  flex: none;
}

.queue-remove {
  width: 28px;
  height: 28px;
  flex: none;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--wp-text-secondary);
  font-size: 10px;
  cursor: pointer;
  opacity: 0;
  transition: opacity var(--duration-fast) var(--ease-standard);
}

.queue-item:hover .queue-remove {
  opacity: 1;
}

.queue-remove:hover {
  background: var(--wp-card-active);
  color: var(--wp-text-primary);
}

/* 面板动画（WinUI Connected Animation 近似） */
.panel-enter-active,
.panel-leave-active {
  transition:
    transform var(--duration-normal) var(--ease-decelerate),
    opacity var(--duration-normal) var(--ease-decelerate);
}

.panel-enter-from,
.panel-leave-to {
  transform: translateY(12px) scale(0.98);
  opacity: 0;
}
</style>
