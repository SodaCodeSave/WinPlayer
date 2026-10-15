<script setup lang="ts">
/**
 * 歌曲列表（WinUI ListView 风格）
 */
import { computed } from "vue";
import { useRouter, type RouteLocationRaw } from "vue-router";
import { usePlayerStore } from "@/stores/player";
import { useLibraryStore } from "@/stores/library";
import { usePluginsStore } from "@/stores/plugins";
import { openContextMenu, type ContextMenuItem } from "@/services/contextMenu";
import { openPlaylistPicker } from "@/services/playlistPicker";
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

/* ------------------------------ 右键菜单 ------------------------------ */

const canAddToPlaylist = computed(
  () =>
    library.playlists.length > 0 ||
    plugins.records.some((r) => r.enabled && plugins.hasCapability(r, "myPlaylists")),
);

/** 行右键菜单：添加到歌单 / 添加到我喜欢 / 移除 / 添加到播放队列 */
function openRowMenu(track: UnifiedTrack, event: MouseEvent) {
  const liked = library.isLiked(track);
  const items: ContextMenuItem[] = [];
  if (canAddToPlaylist.value) {
    items.push({
      key: "playlist",
      label: "添加到歌单",
      icon: "&#xE8FD;",
      action: () => openPlaylistPicker(track),
    });
  }
  items.push({
    key: "like",
    label: liked ? "取消喜欢" : "添加到我喜欢",
    icon: liked ? "&#xE735;" : "&#xE734;",
    action: () => library.toggleLike(track),
  });
  items.push({
    key: "queue",
    label: "添加到播放队列",
    icon: "&#xE710;",
    action: () => addSingle(track),
  });
  if (props.removable) {
    items.push({
      key: "remove",
      label: "移除",
      icon: "&#xE74D;",
      danger: true,
      action: () => emit("remove", track),
    });
  }
  openContextMenu(event, items);
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
      @contextmenu="openRowMenu(row.track, $event)"
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
          @click="openPlaylistPicker(row.track)"
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
</style>
