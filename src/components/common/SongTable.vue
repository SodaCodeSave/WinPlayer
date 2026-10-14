<script setup lang="ts">
/**
 * 歌曲列表（WinUI ListView 风格）
 */
import { computed } from "vue";
import { usePlayerStore } from "@/stores/player";
import { useLibraryStore } from "@/stores/library";
import { usePluginsStore } from "@/stores/plugins";
import { formatDuration, trackArtist, trackTitle } from "@/types/player";
import type { UnifiedTrack } from "@/types/plugin";

const props = withDefaults(
  defineProps<{
    tracks: UnifiedTrack[];
    /** 是否显示来源列（聚合场景） */
    showSource?: boolean;
    /** 列表操作：remove / 无 */
    removable?: boolean;
    emptyText?: string;
  }>(),
  {
    showSource: false,
    removable: false,
    emptyText: "暂无歌曲",
  },
);

const emit = defineEmits<{
  remove: [track: UnifiedTrack];
}>();

const player = usePlayerStore();
const library = useLibraryStore();
const plugins = usePluginsStore();

const rows = computed(() =>
  props.tracks.map((track) => ({
    track,
    key: `${track.sourceId ?? ""}:${track.id}`,
    liked: library.isLiked(track),
    playing: player.currentTrack !== null && `${player.currentTrack.sourceId ?? ""}:${player.currentTrack.id}` === `${track.sourceId ?? ""}:${track.id}`,
    sourceName: track.sourceId
      ? plugins.records.find((r) => r.entry.id === track.sourceId)?.entry.manifest.name ?? track.sourceId
      : "",
  })),
);

function playRow(index: number) {
  void player.playTracks(props.tracks, index);
}

async function toggleLike(track: UnifiedTrack) {
  await library.toggleLike(track);
}

function addSingle(track: UnifiedTrack) {
  player.addToQueue([track]);
}
</script>

<template>
  <div class="song-table">
    <div class="table-header">
      <span class="col-index">#</span>
      <span class="col-title">标题</span>
      <span class="col-artist">艺术家</span>
      <span class="col-album">专辑</span>
      <span v-if="showSource" class="col-source">来源</span>
      <span class="col-actions"></span>
      <span class="col-duration">时长</span>
    </div>

    <div v-if="!rows.length" class="table-empty">{{ emptyText }}</div>

    <div
      v-for="(row, index) in rows"
      :key="row.key"
      class="table-row"
      :class="{ playing: row.playing }"
      @dblclick="playRow(index)"
    >
      <span class="col-index">
        <span v-if="row.playing" class="wp-icon index-playing">&#xE769;</span>
        <span v-else class="index-number">{{ index + 1 }}</span>
        <span class="index-hover wp-icon" @click="playRow(index)">&#xE768;</span>
      </span>
      <span class="col-title">
        <img v-if="row.track.cover" :src="row.track.cover" alt="" class="row-cover" />
        <span class="title-text ellipsis">{{ trackTitle(row.track) }}</span>
      </span>
      <span class="col-artist ellipsis">{{ trackArtist(row.track) }}</span>
      <span class="col-album ellipsis">{{ row.track.album ?? "" }}</span>
      <span v-if="showSource" class="col-source ellipsis">{{ row.sourceName }}</span>
      <span class="col-actions">
        <button
          class="row-action"
          :class="{ liked: row.liked }"
          :title="row.liked ? '取消喜欢' : '添加到我喜欢'"
          @click="toggleLike(row.track)"
        >
          <span class="wp-icon">{{ row.liked ? "&#xE735;" : "&#xE734;" }}</span>
        </button>
        <button class="row-action" title="添加到播放队列" @click="addSingle(row.track)">
          <span class="wp-icon">&#xE710;</span>
        </button>
        <button
          v-if="removable"
          class="row-action danger"
          title="移除"
          @click="emit('remove', row.track)"
        >
          <span class="wp-icon">&#xE74D;</span>
        </button>
      </span>
      <span class="col-duration">{{ formatDuration(row.track.duration) }}</span>
    </div>
  </div>
</template>

<style scoped>
.song-table {
  display: flex;
  flex-direction: column;
}

.table-header {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  height: 36px;
  padding: 0 var(--space-lg);
  border-bottom: 1px solid var(--wp-divider);
  font-size: 12px;
  color: var(--wp-text-secondary);
}

.table-row {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  height: 48px;
  padding: 0 var(--space-lg);
  border-radius: var(--radius-sm);
  font-size: 13px;
  transition: background var(--duration-fast) var(--ease-standard);
  cursor: default;
}

.table-row:hover {
  background: var(--wp-card-hover);
}

.table-row.playing .title-text {
  color: var(--wp-accent);
}

.table-empty {
  padding: var(--space-3xl);
  text-align: center;
  color: var(--wp-text-tertiary);
  font-size: 13px;
}

.col-index {
  width: 32px;
  flex: none;
  text-align: center;
  color: var(--wp-text-tertiary);
  font-size: 12px;
  position: relative;
}

.index-hover {
  display: none;
  cursor: pointer;
  color: var(--wp-text-primary);
  font-size: 12px;
}

.table-row:hover .index-number,
.table-row:hover .index-playing {
  display: none;
}

.table-row:hover .index-hover {
  display: inline-block;
}

.index-playing {
  color: var(--wp-accent);
}

.col-title {
  flex: 2;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: var(--space-lg);
}

.row-cover {
  width: 32px;
  height: 32px;
  border-radius: 3px;
  object-fit: cover;
  flex: none;
}

.title-text {
  min-width: 0;
}

.col-artist {
  flex: 1.4;
  min-width: 0;
  color: var(--wp-text-secondary);
}

.col-album {
  flex: 1.2;
  min-width: 0;
  color: var(--wp-text-secondary);
}

.col-source {
  flex: 0.8;
  min-width: 0;
  color: var(--wp-text-tertiary);
  font-size: 12px;
}

.col-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  width: 104px;
  flex: none;
  justify-content: flex-end;
  opacity: 0;
  transition: opacity var(--duration-fast) var(--ease-standard);
}

.table-row:hover .col-actions {
  opacity: 1;
}

.row-action {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--wp-text-secondary);
  font-size: 12px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.row-action:hover {
  background: var(--wp-card-active);
  color: var(--wp-text-primary);
}

.row-action.liked {
  color: var(--wp-accent);
  opacity: 1;
}

.row-action.danger:hover {
  color: var(--wp-danger);
}

.col-duration {
  width: 48px;
  flex: none;
  text-align: right;
  color: var(--wp-text-tertiary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
