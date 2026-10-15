<script setup lang="ts">
/**
 * 移动端播放条：迷你条 + 全屏播放页
 * - 未播放任何歌曲时整体隐藏
 * - 点击迷你条打开全屏页（封面 / 歌词 / 播放控制），点击行歌词跳转进度
 */
import { computed, nextTick, ref, watch } from "vue";
import CachedImage from "@/components/common/CachedImage.vue";
import { usePlayerStore } from "@/stores/player";
import { useLibraryStore } from "@/stores/library";
import { formatTime, trackArtist, trackTitle } from "@/types/player";

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

/** 全屏页歌词当前行居中滚动（与 LyricsPanel 一致） */
const listRef = ref<HTMLElement | null>(null);

watch(
  () => player.lyricIndex,
  async (index) => {
    if (index < 0 || !showFull.value) return;
    await nextTick();
    const container = listRef.value;
    const line = container?.children[index] as HTMLElement | undefined;
    if (container && line) {
      const target = line.offsetTop - container.clientHeight / 2 + line.clientHeight / 2;
      container.scrollTo({ top: Math.max(0, target), behavior: "smooth" });
    }
  },
);

const progressPercent = computed(() =>
  player.duration > 0 ? Math.min(100, (player.currentTime / player.duration) * 100) : 0,
);
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
        <span v-else class="wp-icon mini-cover-placeholder">&#xE8D6;</span>
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
        <span class="wp-icon">{{ liked ? "&#xE735;" : "&#xE734;" }}</span>
      </button>
      <button
        class="wp-icon-button mini-action"
        :title="player.playing ? '暂停' : '播放'"
        @click.stop="player.togglePlay"
      >
        <span v-if="player.loading" class="wp-icon">&#xE897;</span>
        <span v-else class="wp-icon">{{ player.playing ? "&#xE769;" : "&#xE768;" }}</span>
      </button>
      <button
        class="wp-icon-button mini-action"
        title="下一首"
        :disabled="!player.queue.length"
        @click.stop="player.next()"
      >
        <span class="wp-icon">&#xE893;</span>
      </button>
    </footer>

    <!-- 全屏播放页 -->
    <transition name="full-fade">
      <div v-if="showFull" class="full-player" @click.self="showFull = false">
        <div class="full-header">
          <button class="wp-icon-button" title="收起" @click="showFull = false">
            <span class="wp-icon">&#xE72B;</span>
          </button>
          <span class="full-header-title">正在播放</span>
          <span class="full-header-spacer" />
        </div>

        <div class="full-cover">
          <CachedImage
            v-if="player.currentTrack.cover"
            :src="player.currentTrack.cover"
            class="full-cover-img"
          />
          <span v-else class="wp-icon full-cover-placeholder">&#xE8D6;</span>
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
          <span class="full-time">{{ formatTime(player.currentTime) }}</span>
          <div class="full-progress-track">
            <div class="full-progress-fill" :style="{ width: `${progressPercent}%` }" />
          </div>
          <span class="full-time">{{ formatTime(player.duration) }}</span>
        </div>

        <div class="full-controls">
          <button class="wp-icon-button" :disabled="!player.queue.length" title="上一首" @click="player.prev">
            <span class="wp-icon">&#xE892;</span>
          </button>
          <button
            class="full-play-button"
            :title="player.playing ? '暂停' : '播放'"
            @click="player.togglePlay"
          >
            <span v-if="player.loading" class="wp-icon">&#xE897;</span>
            <span v-else class="wp-icon">{{ player.playing ? "&#xE769;" : "&#xE768;" }}</span>
          </button>
          <button class="wp-icon-button" :disabled="!player.queue.length" title="下一首" @click="player.next()">
            <span class="wp-icon">&#xE893;</span>
          </button>
        </div>
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
  padding-bottom: env(safe-area-inset-bottom);
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

.full-header-spacer {
  width: 40px;
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
  flex: 1;
  height: 3px;
  border-radius: 2px;
  background: var(--wp-control-fill-secondary);
  overflow: hidden;
}

.full-progress-fill {
  height: 100%;
  background: var(--wp-accent);
  transition: width 0.25s linear;
}

.full-controls {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2xl);
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
</style>
