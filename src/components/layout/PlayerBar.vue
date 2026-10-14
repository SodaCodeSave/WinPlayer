<script setup lang="ts">
import { computed, ref } from "vue";
import { usePlayerStore } from "@/stores/player";
import { useLibraryStore } from "@/stores/library";
import { PLAY_MODE_LABEL, formatTime, trackArtist, trackTitle } from "@/types/player";
import WinSlider from "@/components/common/WinSlider.vue";
import QueuePanel from "./QueuePanel.vue";
import LyricsPanel from "./LyricsPanel.vue";

const player = usePlayerStore();
const library = useLibraryStore();

const draggingSeek = ref(false);

const progressMax = computed(() => (player.duration > 0 ? player.duration : 0));
const liked = computed(() => (player.currentTrack ? library.isLiked(player.currentTrack) : false));

const modeIcon = computed(() => {
  switch (player.mode) {
    case "shuffle":
      return "&#xE8B1;";
    case "loop-one":
      return "&#xE8ED;";
    case "loop-all":
      return "&#xE8EE;";
    default:
      return "&#xE768;"; // 顺序播放借用播放图标，避免与循环混淆
  }
});

function onSeekDragEnd(value: number) {
  draggingSeek.value = false;
  player.seek(value);
}

async function toggleLikeCurrent() {
  if (player.currentTrack) await library.toggleLike(player.currentTrack);
}
</script>

<template>
  <footer class="playerbar">
    <!-- 左：当前曲目 -->
    <div class="now-playing">
      <div class="cover">
        <img
          v-if="player.currentTrack?.cover"
          :src="player.currentTrack.cover"
          alt=""
          class="cover-img"
        />
        <span v-else class="wp-icon cover-placeholder">&#xE8D6;</span>
      </div>
      <div class="meta">
        <div class="title ellipsis">{{ player.currentTrack ? trackTitle(player.currentTrack) : "未在播放" }}</div>
        <div class="artist ellipsis">
          {{ player.currentTrack ? trackArtist(player.currentTrack) : "从插件来源发现音乐" }}
        </div>
      </div>
      <button
        class="wp-icon-button like-button"
        :class="{ active: liked }"
        :disabled="!player.currentTrack"
        :title="liked ? '取消喜欢' : '添加到我喜欢'"
        @click="toggleLikeCurrent"
      >
        <span class="wp-icon">{{ liked ? "&#xE735;" : "&#xE734;" }}</span>
      </button>
    </div>

    <!-- 中：控制与进度 -->
    <div class="controls">
      <div class="transport">
        <button
          class="wp-icon-button mode-button"
          :title="PLAY_MODE_LABEL[player.mode]"
          @click="player.cycleMode"
        >
          <span class="wp-icon" v-html="modeIcon" />
        </button>
        <button class="wp-icon-button" title="上一首" :disabled="!player.queue.length" @click="player.prev">
          <span class="wp-icon">&#xE892;</span>
        </button>
        <button
          class="play-button"
          :title="player.playing ? '暂停' : '播放'"
          :disabled="!player.queue.length && !player.currentTrack"
          @click="player.togglePlay"
        >
          <span v-if="player.loading" class="wp-icon">&#xE897;</span>
          <span v-else class="wp-icon">{{ player.playing ? "&#xE769;" : "&#xE768;" }}</span>
        </button>
        <button class="wp-icon-button" title="下一首" :disabled="!player.queue.length" @click="player.next()">
          <span class="wp-icon">&#xE893;</span>
        </button>
        <button
          class="wp-icon-button"
          :class="{ active: player.showLyrics }"
          title="歌词"
          :disabled="!player.currentTrack"
          @click="player.showLyrics = !player.showLyrics; player.showQueue = false"
        >
          <span class="wp-icon">&#xE8D7;</span>
        </button>
      </div>
      <div class="seek-row">
        <span class="time">{{ formatTime(player.currentTime * 1000) }}</span>
        <WinSlider
          class="seek-slider"
          :model-value="player.currentTime"
          :min="0"
          :max="progressMax"
          :step="0.5"
          :disabled="!player.duration"
          :defer-sync="true"
          @update:model-value="player.seek($event)"
          @dragstart="draggingSeek = true"
          @dragend="onSeekDragEnd"
        />
        <span class="time">{{ formatTime(player.duration * 1000) }}</span>
      </div>
    </div>

    <!-- 右：音量与队列 -->
    <div class="extras">
      <button
        class="wp-icon-button"
        :class="{ active: player.showQueue }"
        title="播放队列"
        @click="player.showQueue = !player.showQueue; player.showLyrics = false"
      >
        <span class="wp-icon">&#xE8FD;</span>
      </button>
      <div class="volume-group">
        <button class="wp-icon-button" :title="player.muted ? '取消静音' : '静音'" @click="player.toggleMute">
          <span class="wp-icon">{{ player.muted || player.volume === 0 ? "&#xE74F;" : "&#xE767;" }}</span>
        </button>
        <WinSlider
          class="volume-slider"
          :model-value="player.muted ? 0 : player.volume"
          :min="0"
          :max="1"
          :step="0.01"
          @update:model-value="player.setVolume"
        />
      </div>
    </div>

    <QueuePanel />
    <LyricsPanel />
  </footer>
</template>

<style scoped>
.playerbar {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-xl);
  padding: 0 var(--space-xl);
  background: var(--wp-layer-solid);
  border-top: 1px solid var(--wp-divider);
}

/* ---------------- 左：曲目信息 ---------------- */
.now-playing {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  width: 300px;
  min-width: 0;
}

.cover {
  width: 56px;
  height: 56px;
  border-radius: var(--radius-sm);
  background: var(--wp-control-fill-secondary);
  overflow: hidden;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-card);
}

.cover-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.cover-placeholder {
  font-size: 22px;
  color: var(--wp-text-tertiary);
}

.meta {
  min-width: 0;
  flex: 1;
}

.title {
  font-size: 14px;
  font-weight: 600;
}

.artist {
  font-size: 12px;
  color: var(--wp-text-secondary);
  margin-top: 2px;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.like-button {
  flex: none;
  width: 32px;
  height: 32px;
}

/* ---------------- 中：控制区 ---------------- */
.controls {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  min-width: 0;
  max-width: 640px;
  margin: 0 auto;
}

.transport {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.mode-button {
  width: 32px;
  height: 32px;
  font-size: 15px;
}

.play-button {
  width: 40px;
  height: 40px;
  border: none;
  border-radius: 50%;
  background: var(--wp-accent);
  color: var(--wp-text-on-accent);
  font-size: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  margin: 0 var(--space-sm);
  transition: background var(--duration-fast) var(--ease-standard);
}

.play-button:hover {
  background: var(--wp-accent-hover);
}

.play-button:active {
  background: var(--wp-accent-active);
}

.play-button:disabled {
  background: var(--wp-control-fill-secondary);
  color: var(--wp-text-disabled);
  cursor: default;
}

.seek-row {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  width: 100%;
}

.time {
  font-size: 11px;
  color: var(--wp-text-secondary);
  font-variant-numeric: tabular-nums;
  width: 40px;
  text-align: center;
  flex: none;
}

.seek-slider {
  flex: 1;
}

/* ---------------- 右：音量/队列 ---------------- */
.extras {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  width: 300px;
  justify-content: flex-end;
  flex: none;
}

.volume-group {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.volume-slider {
  width: 96px;
}
</style>
