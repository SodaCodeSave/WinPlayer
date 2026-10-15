<script setup lang="ts">
/**
 * 通用媒体卡片（歌单 / 专辑 / 歌手）
 *
 * 三种卡片共用同一套栅格尺寸与 hover 行为，区别只在封面形状和副标题：
 * - 歌单：方形封面，副标题「N 首 · 来源」
 * - 专辑：方形封面，副标题为艺术家名
 * - 歌手：圆形头像，副标题「N 首 · N 张专辑」
 *
 * 点击一律跳转到对应的详情页（插件来源歌单 / 专辑 / 歌手），
 * 没有 sourceId 的条目不提供跳转（保持可点击但无动作）。
 */
import { computed } from "vue";
import { useRouter } from "vue-router";
import { pluginDisplayName, usePluginsStore } from "@/stores/plugins";
import type { UnifiedAlbum, UnifiedArtist, UnifiedPlaylist } from "@/types/plugin";

const props = defineProps<{
  kind: "playlist" | "album" | "artist";
  playlist?: UnifiedPlaylist;
  album?: UnifiedAlbum;
  artist?: UnifiedArtist;
}>();

const router = useRouter();
const plugins = usePluginsStore();

const playlist = computed(() => (props.kind === "playlist" ? props.playlist : undefined));
const album = computed(() => (props.kind === "album" ? props.album : undefined));
const artist = computed(() => (props.kind === "artist" ? props.artist : undefined));

const item = computed(() => playlist.value ?? album.value ?? artist.value);
const sourceId = computed(() => item.value?.sourceId);

// 三种类型的封面字段名不同（歌手是 avatar，其余是 cover），分开取以避免联合类型告警
const cover = computed(() => {
  if (artist.value) return artist.value.avatar ?? "";
  if (album.value) return album.value.cover ?? "";
  return playlist.value?.cover ?? "";
});

const title = computed(() => item.value?.name || "未命名");
const sourceName = computed(() =>
  sourceId.value ? pluginDisplayName(plugins.records, sourceId.value) : "本地",
);

const placeholderIcon = computed(() => (props.kind === "artist" ? "\uE77B" : "\uE8FD"));

const subtitle = computed(() => {
  if (album.value) {
    const names = (album.value.artistNames ?? []).filter(Boolean);
    if (names.length) return names.join(" / ");
    if (album.value.trackCount) return `${album.value.trackCount} 首`;
    return sourceName.value;
  }
  if (artist.value) {
    const parts: string[] = [];
    if (artist.value.songCount) parts.push(`${artist.value.songCount} 首`);
    if (artist.value.albumCount) parts.push(`${artist.value.albumCount} 张专辑`);
    return parts.length ? parts.join(" · ") : sourceName.value;
  }
  const parts: string[] = [];
  if (playlist.value?.trackCount) parts.push(`${playlist.value.trackCount} 首`);
  parts.push(sourceName.value);
  return parts.join(" · ");
});

const tooltip = computed(() => {
  const text = playlist.value?.description ?? album.value?.description ?? artist.value?.briefDesc;
  return text || title.value;
});

const target = computed(() => {
  const source = sourceId.value;
  const id = item.value?.id;
  if (!source || !id) return null;
  if (props.kind === "album") return { name: "album", params: { source, id } };
  if (props.kind === "artist") return { name: "artist", params: { source, id } };
  return { name: "sourcePlaylist", params: { source, id } };
});

function open() {
  if (!target.value) return;
  void router.push(target.value);
}
</script>

<template>
  <button class="media-card" :title="tooltip" @click="open">
    <div class="card-cover" :class="{ round: kind === 'artist' }">
      <img v-if="cover" :src="cover" alt="" class="cover-img" />
      <span v-else class="wp-icon cover-placeholder">{{ placeholderIcon }}</span>
    </div>
    <div class="card-name ellipsis">{{ title }}</div>
    <div class="card-sub ellipsis">{{ subtitle }}</div>
  </button>
</template>

<style scoped>
.media-card {
  width: 160px;
  flex: none;
  border: none;
  background: transparent;
  padding: var(--space-md);
  border-radius: var(--radius-lg);
  cursor: pointer;
  text-align: left;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  transition: background var(--duration-fast) var(--ease-standard);
}

.media-card:hover {
  background: var(--wp-card-hover);
}

.card-cover {
  width: 100%;
  aspect-ratio: 1;
  border-radius: var(--radius-md);
  background: var(--wp-control-fill-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  box-shadow: var(--shadow-card);
}

/* 歌手用圆形头像，和方形的歌单/专辑封面区分开 */
.card-cover.round {
  border-radius: 50%;
}

.cover-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.cover-placeholder {
  font-size: 36px;
  color: var(--wp-text-tertiary);
}

.card-name {
  font-size: 13px;
  font-weight: 600;
}

.card-sub {
  font-size: 11px;
  color: var(--wp-text-secondary);
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
