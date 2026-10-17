<script setup lang="ts">
/**
 * 移动端播放条：迷你条 + 全屏播放页
 * - 未播放任何歌曲时整体隐藏
 * - 点击迷你条打开全屏页（封面 / 歌词 / 播放控制），点击行歌词跳转进度
 */
import { computed, ref, watch } from "vue";
import CachedImage from "@/components/common/CachedImage.vue";
import { useAutoCenter } from "@/composables/useAutoCenter";
import { usePlayerStore } from "@/stores/player";
import { useLibraryStore } from "@/stores/library";
import {
  PLAY_MODE_LABEL,
  formatTime,
  playModeIcon,
  playModeIconFilled,
  trackArtist,
  trackTitle,
} from "@/types/player";

const player = usePlayerStore();
const library = useLibraryStore();

const showFull = ref(false);

const liked = computed(() => (player.currentTrack ? library.isLiked(player.currentTrack) : false));

async function toggleLikeCurrent() {
  if (player.currentTrack) await library.toggleLike(player.currentTrack);
}

function openFull() {
  if (player.currentTrack) showFull.value = true;
}

/** 全屏页歌词当前行自动居中；用户手动滚动后延迟回正 */
const listRef = ref<HTMLElement | null>(null);

useAutoCenter({
  container: listRef,
  itemSelector: ".full-lrc-line",
  getIndex: () => player.lyricIndex,
  isActive: () => showFull.value,
});

const progressPercent = computed(() =>
  player.duration > 0 ? Math.min(100, (player.currentTime / player.duration) * 100) : 0,
);

/* ------------------------------ 播放模式 ------------------------------ */

const modeIcon = computed(() => playModeIcon(player.mode));
const modeIconFilled = computed(() => playModeIconFilled(player.mode));
const modeLabel = computed(() => PLAY_MODE_LABEL[player.mode]);

/* ------------------------------ 队列面板 ------------------------------ */

const showQueue = ref(false);

// 收起全屏页时一并关闭队列面板，避免下次打开时残留
watch(showFull, (open) => {
  if (!open) showQueue.value = false;
});

/* ------------------------------ 进度条拖动 ------------------------------ */

const trackRef = ref<HTMLElement | null>(null);
const dragging = ref(false);
const dragPercent = ref(0);

/** 拖动时显示手指位置，否则显示播放进度 */
const displayedPercent = computed(() => (dragging.value ? dragPercent.value : progressPercent.value));
const displayedTime = computed(() =>
  dragging.value ? formatTime((player.duration * dragPercent.value) / 100) : formatTime(player.currentTime),
);

function percentFromEvent(event: PointerEvent): number {
  const el = trackRef.value;
  if (!el) return 0;
  const rect = el.getBoundingClientRect();
  if (rect.width <= 0) return 0;
  return Math.min(100, Math.max(0, ((event.clientX - rect.left) / rect.width) * 100));
}

function onProgressDown(event: PointerEvent): void {
  if (player.duration <= 0) return;
  dragging.value = true;
  dragPercent.value = percentFromEvent(event);
  // 合成事件 / 无效 pointerId 时 setPointerCapture 会抛错，忽略即可
  try {
    trackRef.value?.setPointerCapture(event.pointerId);
  } catch {
    /* 忽略：无法捕获时仍可在轨道内拖动 */
  }
}

function onProgressMove(event: PointerEvent): void {
  if (!dragging.value) return;
  dragPercent.value = percentFromEvent(event);
}

function onProgressUp(event: PointerEvent): void {
  if (!dragging.value) return;
  dragging.value = false;
  player.seek((player.duration * percentFromEvent(event)) / 100);
}
</script>

<template>
  <template v-if="player.currentTrack">
    <!-- 迷你播放条 -->
    <footer class="mini-bar" @click="openFull">
      <div class="mini-progress" :style="{ width: `${progressPercent}%` }" />
      <div class="mini-cover">
        <CachedImage
          v-if="player.currentTrack.cover"
          :src="player.currentTrack.cover"
          class="mini-cover-img"
        />
        <span v-else class="wp-icon mini-cover-placeholder">&#xE852;</span>
      </div>
      <div class="mini-meta">
        <div class="mini-title ellipsis">{{ trackTitle(player.currentTrack) }}</div>
        <div class="mini-artist ellipsis">{{ trackArtist(player.currentTrack) }}</div>
      </div>
      <button
        class="wp-icon-button mini-action"
        :title="liked ? '取消喜欢' : '添加到我喜欢'"
        @click.stop="toggleLikeCurrent"
      >
        <span class="wp-icon" :class="{ filled: liked }">{{ liked ? "&#xF47E;" : "&#xF47A;" }}</span>
      </button>
      <button
        class="wp-icon-button mini-action"
        :title="player.playing ? '暂停' : '播放'"
        @click.stop="player.togglePlay"
      >
        <span v-if="player.loading" class="wp-icon">&#xF709;</span>
        <span v-else class="wp-icon filled">{{ player.playing ? "&#xF5AC;" : "&#xF610;" }}</span>
      </button>
      <button
        class="wp-icon-button mini-action"
        title="下一首"
        :disabled="!player.queue.length"
        @click.stop="player.next()"
      >
        <span class="wp-icon filled">&#xF574;</span>
      </button>
    </footer>

    <!-- 全屏播放页 -->
    <transition name="full-fade">
      <div v-if="showFull" class="full-player" @click.self="showFull = false">
        <div class="full-header">
          <button class="wp-icon-button" title="收起" @click="showFull = false">
            <span class="wp-icon">&#xF2AB;</span>
          </button>
          <span class="full-header-title">正在播放</span>
          <button
            class="wp-icon-button"
            :title="liked ? '取消喜欢' : '添加到我喜欢'"
            @click="toggleLikeCurrent"
          >
            <span class="wp-icon" :class="{ filled: liked }">{{ liked ? "&#xF47E;" : "&#xF47A;" }}</span>
          </button>
        </div>

        <div class="full-cover">
          <CachedImage
            v-if="player.currentTrack.cover"
            :src="player.currentTrack.cover"
            class="full-cover-img"
          />
          <span v-else class="wp-icon full-cover-placeholder">&#xE852;</span>
        </div>

        <div class="full-meta">
          <div class="full-title ellipsis">{{ trackTitle(player.currentTrack) }}</div>
          <div class="full-artist ellipsis">{{ trackArtist(player.currentTrack) }}</div>
        </div>

        <div ref="listRef" class="full-lyrics">
          <div v-if="player.lyricsLoading" class="lyrics-empty">歌词加载中…</div>
          <div v-else-if="!player.lyrics.length" class="lyrics-empty">暂无歌词</div>
          <template v-else>
            <p
              v-for="(line, index) in player.lyrics"
              :key="index"
              class="full-lrc-line"
              :class="{ active: index === player.lyricIndex }"
              @click="player.seek(line.time / 1000)"
            >
              {{ line.text }}
            </p>
          </template>
        </div>

        <div class="full-progress">
          <span class="full-time">{{ displayedTime }}</span>
          <div
            ref="trackRef"
            class="full-progress-track"
            @pointerdown="onProgressDown"
            @pointermove="onProgressMove"
            @pointerup="onProgressUp"
            @pointercancel="onProgressUp"
          >
            <div class="full-progress-fill" :style="{ width: `${displayedPercent}%` }" />
            <div
              class="full-progress-thumb"
              :class="{ dragging }"
              :style="{ left: `${displayedPercent}%` }"
            />
          </div>
          <span class="full-time">{{ formatTime(player.duration) }}</span>
        </div>

        <div class="full-controls">
          <button class="wp-icon-button" :title="modeLabel" @click="player.cycleMode">
            <span class="wp-icon" :class="{ filled: modeIconFilled }" v-html="modeIcon" />
          </button>
          <button class="wp-icon-button" :disabled="!player.queue.length" title="上一首" @click="player.prev">
            <span class="wp-icon filled">&#xF633;</span>
          </button>
          <button
            class="full-play-button"
            :title="player.playing ? '暂停' : '播放'"
            @click="player.togglePlay"
          >
            <span v-if="player.loading" class="wp-icon">&#xF709;</span>
            <span v-else class="wp-icon filled">{{ player.playing ? "&#xF5AC;" : "&#xF610;" }}</span>
          </button>
          <button class="wp-icon-button" :disabled="!player.queue.length" title="下一首" @click="player.next()">
            <span class="wp-icon filled">&#xF574;</span>
          </button>
          <button class="wp-icon-button" title="播放队列" :disabled="!player.queue.length" @click="showQueue = true">
            <span class="wp-icon">&#xF4EE;</span>
          </button>
        </div>

        <!-- 播放队列（底部抽屉） -->
        <transition name="full-fade">
          <div v-if="showQueue" class="full-queue" @click.self="showQueue = false">
            <div class="queue-sheet">
              <div class="queue-sheet-header">
                <span class="queue-sheet-title">播放队列</span>
                <span class="queue-sheet-count">{{ player.queue.length }} 首</span>
                <button v-if="player.queue.length" class="queue-sheet-clear" @click="player.clearQueue()">
                  清空
                </button>
                <button class="wp-icon-button" title="关闭" @click="showQueue = false">
                  <span class="wp-icon">&#xF36A;</span>
                </button>
              </div>
              <div class="queue-sheet-body">
                <div v-if="!player.queue.length" class="queue-sheet-empty">队列为空，去发现音乐吧</div>
                <ul v-else class="queue-sheet-list">
                  <li
                    v-for="(item, index) in player.queue"
                    :key="item.uid"
                    class="queue-sheet-item"
                    :class="{ current: index === player.currentIndex }"
                    @click="player.playAt(index)"
                  >
                    <span class="queue-sheet-index">
                      <span v-if="index === player.currentIndex" class="wp-icon filled queue-sheet-playing">&#xE855;</span>
                      <span v-else>{{ index + 1 }}</span>
                    </span>
                    <span class="queue-sheet-meta">
                      <span class="queue-sheet-name ellipsis">{{ trackTitle(item.track) }}</span>
                      <span class="queue-sheet-artist ellipsis">{{ trackArtist(item.track) }}</span>
                    </span>
                    <button
                      class="wp-icon-button queue-sheet-remove"
                      title="移除"
                      @click.stop="player.removeFromQueue(item.uid)"
                    >
                      <span class="wp-icon">&#xF36A;</span>
                    </button>
                  </li>
                </ul>
              </div>
            </div>
          </div>
        </transition>
      </div>
    </transition>
  </template>
</template>

<style scoped>
/* ---------------- 迷你条 ---------------- */
.mini-bar {
  position: relative;
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--space-md);
  height: 56px;
  padding: 0 var(--space-md);
  background: var(--wp-layer-solid);
  border-top: 1px solid var(--wp-divider);
  cursor: pointer;
  z-index: 31;
}

/* 顶部细进度线 */
.mini-progress {
  position: absolute;
  top: -1px;
  left: 0;
  height: 2px;
  background: var(--wp-accent);
  transition: width 0.25s linear;
  pointer-events: none;
}

.mini-cover {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-sm);
  background: var(--wp-control-fill-secondary);
  overflow: hidden;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.mini-cover-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.mini-cover-placeholder {
  font-size: 16px;
  color: var(--wp-text-tertiary);
}

.mini-meta {
  flex: 1;
  min-width: 0;
}

.mini-title {
  font-size: 13px;
  font-weight: 600;
}

.mini-artist {
  font-size: 11px;
  color: var(--wp-text-secondary);
  margin-top: 1px;
}

.mini-action {
  width: 36px;
  height: 36px;
  flex: none;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ---------------- 全屏播放页 ---------------- */
.full-player {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  background: var(--wp-base);
  z-index: 100;
  /* 全屏浮层脱离 app-shell 内边距：自行避开状态栏与底部导航栏 */
  padding-top: var(--safe-area-top);
  padding-bottom: var(--safe-area-bottom);
}

.full-fade-enter-active,
.full-fade-leave-active {
  transition: opacity var(--duration-normal) var(--ease-decelerate);
}

.full-fade-enter-from,
.full-fade-leave-to {
  opacity: 0;
}

.full-header {
  flex: none;
  display: flex;
  align-items: center;
  padding: var(--space-md);
}

.full-header-title {
  flex: 1;
  text-align: center;
  font-size: 14px;
  font-weight: 600;
}

.full-cover {
  flex: none;
  align-self: center;
  width: min(72vw, 300px);
  aspect-ratio: 1;
  margin: var(--space-lg) 0;
  border-radius: var(--radius-lg);
  background: var(--wp-control-fill-secondary);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-card);
}

.full-cover-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.full-cover-placeholder {
  font-size: 64px;
  color: var(--wp-text-tertiary);
}

.full-meta {
  flex: none;
  padding: 0 var(--space-xl);
  text-align: center;
}

.full-title {
  font-size: 18px;
  font-weight: 600;
}

.full-artist {
  font-size: 13px;
  color: var(--wp-text-secondary);
  margin-top: 4px;
}

.full-lyrics {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: var(--space-2xl) var(--space-xl);
}

.lyrics-empty {
  padding: var(--space-3xl) 0;
  text-align: center;
  color: var(--wp-text-tertiary);
  font-size: 13px;
}

.full-lrc-line {
  margin: 0;
  padding: var(--space-sm) 0;
  font-size: 15px;
  line-height: 1.6;
  color: var(--wp-text-tertiary);
  cursor: pointer;
  text-align: center;
  transition: color var(--duration-normal) var(--ease-standard);
}

.full-lrc-line.active {
  color: var(--wp-text-primary);
  font-weight: 600;
  font-size: 17px;
}

.full-progress {
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-md) var(--space-xl) 0;
}

.full-time {
  font-size: 11px;
  color: var(--wp-text-secondary);
  font-variant-numeric: tabular-nums;
  width: 40px;
  text-align: center;
  flex: none;
}

.full-progress-track {
  position: relative;
  flex: 1;
  /* 可见线只有 4px，但把整体高度撑到 24px 方便手指点按；
     背景只在正中画 4px 的线（border-box 下 padding 吃掉多余高度）。 */
  height: 24px;
  padding: 10px 0;
  background-image: linear-gradient(var(--wp-control-fill-secondary), var(--wp-control-fill-secondary));
  background-repeat: no-repeat;
  background-position: center;
  background-size: 100% 4px;
  cursor: pointer;
  /* 拖动进度条时不触发页面滚动 */
  touch-action: none;
}

.full-progress-fill {
  height: 100%;
  border-radius: 2px;
  background: var(--wp-accent);
  transition: width 0.25s linear;
}

/* 拖动时才显示，避免静止时干扰视觉 */
.full-progress-thumb {
  position: absolute;
  top: 50%;
  width: 14px;
  height: 14px;
  margin-left: -7px;
  border-radius: 50%;
  background: var(--wp-accent);
  box-shadow: var(--shadow-card);
  transform: translateY(-50%);
  opacity: 0;
  transition: opacity var(--duration-fast) var(--ease-standard);
  pointer-events: none;
}

.full-progress-thumb.dragging {
  opacity: 1;
}

.full-controls {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-lg);
  padding: var(--space-lg) 0 var(--space-xl);
}

.full-play-button {
  width: 64px;
  height: 64px;
  border: none;
  border-radius: 50%;
  background: var(--wp-accent);
  color: var(--wp-text-on-accent);
  font-size: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-standard);
}

.full-play-button:active {
  background: var(--wp-accent-active);
}

/* ---------------- 播放队列（底部抽屉） ---------------- */
.full-queue {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  background: rgba(0, 0, 0, 0.4);
  z-index: 1;
}

.queue-sheet {
  display: flex;
  flex-direction: column;
  max-height: 72%;
  background: var(--wp-flyout);
  border-top-left-radius: var(--radius-xl);
  border-top-right-radius: var(--radius-xl);
  padding-bottom: var(--safe-area-bottom);
  box-shadow: var(--shadow-flyout);
}

.queue-sheet-header {
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-lg) var(--space-lg) var(--space-md) var(--space-xl);
  border-bottom: 1px solid var(--wp-divider);
}

.queue-sheet-title {
  font-size: 15px;
  font-weight: 600;
}

.queue-sheet-count {
  flex: 1;
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.queue-sheet-clear {
  border: none;
  background: transparent;
  color: var(--wp-accent);
  font-size: 13px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.queue-sheet-clear:active {
  background: var(--wp-card-active);
}

.queue-sheet-body {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: var(--space-sm) var(--space-sm) var(--space-lg);
}

.queue-sheet-empty {
  padding: var(--space-3xl) var(--space-xl);
  text-align: center;
  color: var(--wp-text-tertiary);
  font-size: 13px;
}

.queue-sheet-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.queue-sheet-item {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  min-height: 52px;
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-md);
}

.queue-sheet-item:active {
  background: var(--wp-card-active);
}

.queue-sheet-item.current .queue-sheet-name {
  color: var(--wp-accent);
}

.queue-sheet-index {
  width: 28px;
  flex: none;
  text-align: center;
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.queue-sheet-playing {
  color: var(--wp-accent);
  font-size: 12px;
}

.queue-sheet-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.queue-sheet-name {
  font-size: 14px;
}

.queue-sheet-artist {
  font-size: 12px;
  color: var(--wp-text-secondary);
  margin-top: 2px;
}

.queue-sheet-remove {
  flex: none;
  width: 32px;
  height: 32px;
}
</style>
