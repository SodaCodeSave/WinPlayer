<script setup lang="ts">
/**
 * 歌曲列表（WinUI ListView 风格）
 */
import { computed, ref } from "vue";
import { useRouter, type RouteLocationRaw } from "vue-router";
import { ContentDialog } from "winuionweb";
import { usePlayerStore } from "@/stores/player";
import { useLibraryStore } from "@/stores/library";
import { usePluginsStore } from "@/stores/plugins";
import { toast } from "@/services/toast";
import { formatDuration, trackTitle } from "@/types/player";
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
const router = useRouter();

/** 歌手/专辑名是否能跳转，取决于插件有没有回传对应的 id */
interface CellLink {
  name: string;
  target: RouteLocationRaw | null;
}

function artistCells(track: UnifiedTrack): CellLink[] {
  const names = track.artists ?? [];
  const ids = track.artistIds ?? [];
  return names.map((name, index) => ({
    name,
    target:
      track.sourceId && ids[index]
        ? { name: "artist", params: { source: track.sourceId, id: ids[index] } }
        : null,
  }));
}

function albumCell(track: UnifiedTrack): CellLink | null {
  if (!track.album) return null;
  return {
    name: track.album,
    target:
      track.sourceId && track.albumId
        ? { name: "album", params: { source: track.sourceId, id: track.albumId } }
        : null,
  };
}

const rows = computed(() =>
  props.tracks.map((track) => ({
    track,
    key: `${track.sourceId ?? ""}:${track.id}`,
    liked: library.isLiked(track),
    playing: player.currentTrack !== null && `${player.currentTrack.sourceId ?? ""}:${player.currentTrack.id}` === `${track.sourceId ?? ""}:${track.id}`,
    sourceName: track.sourceId
      ? plugins.records.find((r) => r.entry.id === track.sourceId)?.entry.manifest.name ?? track.sourceId
      : "",
    artists: artistCells(track),
    album: albumCell(track),
  })),
);

function playRow(index: number) {
  void player.playTracks(props.tracks, index);
}

function go(target: RouteLocationRaw) {
  void router.push(target);
}

/** 歌手/专辑名上双击时只允许跳转，不要顺带触发整行播放 */
function noop() {
  /* 仅用于拦截事件 */
}

async function toggleLike(track: UnifiedTrack) {
  await library.toggleLike(track);
}

function addSingle(track: UnifiedTrack) {
  player.addToQueue([track]);
}

/* ------------------------------ 添加到歌单 ------------------------------ */

/** 歌单选项（本地歌单 + 各插件的平台个人歌单） */
interface PlaylistOption {
  key: string;
  name: string;
  meta: string;
  kind: "local" | "remote";
  playlistId: string;
  sourceId?: string;
}

interface PickerDialogRef {
  ShowAsync: () => Promise<string>;
  Hide: () => void;
}

const addDialogRef = ref<PickerDialogRef | null>(null);
const pendingTrack = ref<UnifiedTrack | null>(null);
const pickerLoading = ref(false);
const pickerBusy = ref(false);
const pickerOptions = ref<PlaylistOption[]>([]);

const canAddToPlaylist = computed(
  () =>
    library.playlists.length > 0 ||
    plugins.records.some((r) => r.enabled && plugins.hasCapability(r, "myPlaylists")),
);

async function openAddDialog(track: UnifiedTrack) {
  pendingTrack.value = track;
  pickerOptions.value = library.playlists.map((p) => ({
    key: `local:${p.id}`,
    name: p.name,
    meta: p.trackCount ? `${p.trackCount} 首 · 本地` : "本地歌单",
    kind: "local",
    playlistId: String(p.id),
  }));
  pickerLoading.value = plugins.records.some((r) => r.enabled && plugins.hasCapability(r, "myPlaylists"));
  pickerBusy.value = false;

  const dialog = addDialogRef.value;
  if (!dialog) return;
  const shown = dialog.ShowAsync();
  // 平台歌单实时拉取（单个插件失败不影响本地歌单可选）
  try {
    const groups = await plugins.getUserPlaylistsAll();
    const remoteOptions: PlaylistOption[] = [];
    for (const group of groups) {
      for (const p of group.result ?? []) {
        remoteOptions.push({
          key: `${group.record.entry.id}:${p.id}`,
          name: p.name,
          meta: [
            group.record.entry.manifest.name,
            p.trackCount ? `${p.trackCount} 首` : "",
          ]
            .filter(Boolean)
            .join(" · "),
          kind: "remote",
          playlistId: p.id,
          sourceId: group.record.entry.id,
        });
      }
    }
    pickerOptions.value = [...pickerOptions.value, ...remoteOptions];
  } finally {
    pickerLoading.value = false;
  }
  await shown;
}

async function pickPlaylist(option: PlaylistOption) {
  const track = pendingTrack.value;
  if (!track || pickerBusy.value) return;
  pickerBusy.value = true;
  try {
    if (option.kind === "local") {
      await library.addTracksToPlaylist(Number(option.playlistId), [track]);
    } else if (option.sourceId) {
      await plugins.addToUserPlaylist(option.sourceId, option.playlistId, track.id);
    }
    toast(`已添加到「${option.name}」`, "success");
    addDialogRef.value?.Hide();
  } catch (error) {
    toast(error instanceof Error ? error.message : String(error), "error");
  } finally {
    pickerBusy.value = false;
  }
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
        <span class="title-main">
          <span class="title-text ellipsis">{{ trackTitle(row.track) }}</span>
          <span v-if="row.track.alias" class="title-alias ellipsis">（{{ row.track.alias }}）</span>
        </span>
        <span v-if="row.track.vip" class="title-vip">VIP</span>
      </span>
      <span class="col-artist ellipsis">
        <template v-for="(artist, index) in row.artists" :key="`${artist.name}-${index}`">
          <span v-if="index" class="cell-sep"> / </span>
          <button
            v-if="artist.target"
            class="cell-link"
            :title="`查看歌手「${artist.name}」`"
            @click.stop="go(artist.target)"
            @dblclick.stop="noop"
          >
            {{ artist.name }}
          </button>
          <span v-else>{{ artist.name }}</span>
        </template>
      </span>
      <span class="col-album ellipsis">
        <button
          v-if="row.album?.target"
          class="cell-link"
          :title="`查看专辑「${row.album.name}」`"
          @click.stop="go(row.album.target)"
          @dblclick.stop
        >
          {{ row.album.name }}
        </button>
        <template v-else>{{ row.album?.name ?? "" }}</template>
      </span>
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
        <button
          v-if="canAddToPlaylist"
          class="row-action"
          title="添加到歌单"
          @click="openAddDialog(row.track)"
        >
          <span class="wp-icon">&#xE8FD;</span>
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

  <ContentDialog ref="addDialogRef" Title="添加到歌单" CloseButtonText="取消">
    <div class="picker">
      <div v-if="pendingTrack" class="picker-track ellipsis">{{ trackTitle(pendingTrack) }}</div>
      <div v-if="pickerLoading" class="picker-hint">正在加载平台歌单…</div>
      <div v-else-if="!pickerOptions.length" class="picker-hint">还没有可用的歌单</div>
      <div v-else class="picker-list">
        <button
          v-for="option in pickerOptions"
          :key="option.key"
          class="picker-item"
          :disabled="pickerBusy"
          @click="pickPlaylist(option)"
        >
          <span class="wp-icon picker-icon">&#xE8FD;</span>
          <span class="picker-name ellipsis">{{ option.name }}</span>
          <span class="picker-meta ellipsis">{{ option.meta }}</span>
        </button>
      </div>
    </div>
  </ContentDialog>
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

.title-main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.title-text {
  flex: 0 1 auto;
  min-width: 0;
}

/* 别名用浅一号的字体；宽度不足时先于标题收缩（flex-shrink 更大），保证 VIP 标识始终可见 */
.title-alias {
  flex: 0 999 auto;
  min-width: 0;
  color: var(--wp-text-secondary);
  font-size: 12px;
}

.title-vip {
  flex: none;
  font-size: 10px;
  line-height: 1;
  padding: 2px 4px;
  border: 1px solid var(--wp-accent);
  border-radius: 3px;
  color: var(--wp-accent);
  font-weight: 600;
  letter-spacing: 0.5px;
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

/* 歌手名 / 专辑名：有对应 id 时是可点击的跳转入口 */
.cell-link {
  border: none;
  background: transparent;
  padding: 0;
  font: inherit;
  color: inherit;
  cursor: pointer;
  border-radius: 3px;
}

.cell-link:hover {
  color: var(--wp-text-primary);
  text-decoration: underline;
}

.cell-sep {
  color: var(--wp-text-tertiary);
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

/* 添加到歌单选择器 */
.picker {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  min-width: 320px;
}

.picker-track {
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.picker-hint {
  font-size: 13px;
  color: var(--wp-text-secondary);
  padding: var(--space-lg) 0;
}

.picker-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 320px;
  overflow-y: auto;
}

.picker-item {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  height: 36px;
  padding: 0 var(--space-md);
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  font: inherit;
  color: var(--wp-text-primary);
  cursor: pointer;
  text-align: left;
  transition: background var(--duration-fast) var(--ease-standard);
}

.picker-item:hover:not(:disabled) {
  background: var(--wp-card-hover);
}

.picker-item:disabled {
  opacity: 0.5;
  cursor: default;
}

.picker-icon {
  font-size: 14px;
  color: var(--wp-text-secondary);
  flex: none;
}

.picker-name {
  flex: 1;
  min-width: 0;
  font-size: 13px;
}

.picker-meta {
  flex: none;
  max-width: 45%;
  font-size: 12px;
  color: var(--wp-text-tertiary);
}
</style>
