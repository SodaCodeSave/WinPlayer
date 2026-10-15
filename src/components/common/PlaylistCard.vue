<script setup lang="ts">
/**
 * 歌单卡片（推荐/搜索结果用）。
 *
 * 歌单卡现在是「媒体卡片」的一种，实现统一收拢在 MediaCard.vue，
 * 保留本文件只是为了让既有的 <PlaylistCard> 调用点无需改动。
 */
import MediaCard from "@/components/common/MediaCard.vue";
import type { UnifiedPlaylist } from "@/types/plugin";

defineProps<{
  playlist: UnifiedPlaylist;
}>();
</script>

<template>
  <MediaCard kind="playlist" :playlist="playlist" />
</template>
