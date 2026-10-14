<script setup lang="ts">
/** 歌单卡片（推荐/搜索结果用） */
import { usePluginsStore } from "@/stores/plugins";
import { useLibraryStore } from "@/stores/library";
import type { UnifiedPlaylist } from "@/types/plugin";

const props = defineProps<{
  playlist: UnifiedPlaylist;
}>();

const plugins = usePluginsStore();
const library = useLibraryStore();

const sourceName = props.playlist.sourceId
  ? plugins.records.find((r) => r.entry.id === props.playlist.sourceId)?.entry.manifest.name ??
    props.playlist.sourceId
  : "本地";

async function open() {
  if (!props.playlist.sourceId) return;
  // 插件来源歌单：当前版本以“添加全部到我喜欢”支持，完整歌单详情页预留
  try {
    const detail = await plugins.getPlaylistDetail(props.playlist.sourceId, props.playlist.id);
    if (detail?.tracks?.length) {
      for (const track of detail.tracks) {
        if (!library.isLiked(track)) await library.toggleLike(track);
      }
    }
  } catch {
    /* 由 store toast 提示 */
  }
}
</script>

<template>
  <button class="playlist-card" :title="playlist.description || playlist.name" @click="open">
    <div class="card-cover">
      <img v-if="playlist.cover" :src="playlist.cover" alt="" class="cover-img" />
      <span v-else class="wp-icon cover-placeholder">&#xE8FD;</span>
    </div>
    <div class="card-name ellipsis">{{ playlist.name }}</div>
    <div class="card-sub">
      <span v-if="playlist.trackCount" class="count">{{ playlist.trackCount }} 首 · </span>{{ sourceName }}
    </div>
  </button>
</template>

<style scoped>
.playlist-card {
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

.playlist-card:hover {
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
