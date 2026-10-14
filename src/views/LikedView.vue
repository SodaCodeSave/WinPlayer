<script setup lang="ts">
import { useRouter } from "vue-router";
import { useLibraryStore } from "@/stores/library";
import { usePlayerStore } from "@/stores/player";
import SongTable from "@/components/common/SongTable.vue";
import EmptyState from "@/components/common/EmptyState.vue";

const router = useRouter();
const library = useLibraryStore();
const player = usePlayerStore();

function playAll() {
  if (library.likedTracks.length) void player.playTracks(library.likedTracks, 0);
}

function shufflePlay() {
  if (library.likedTracks.length) {
    void player.playTracks(
      library.likedTracks,
      Math.floor(Math.random() * library.likedTracks.length),
    );
  }
}
</script>

<template>
  <div class="wp-page">
    <template v-if="library.liked.length">
      <div class="wp-page-header">
        <div class="header-cover">
          <span class="wp-icon">&#xE735;</span>
        </div>
        <div class="header-meta">
          <h1 class="wp-page-title">我喜欢的音乐</h1>
          <div class="header-sub">{{ library.liked.length }} 首</div>
        </div>
        <div class="header-actions">
          <button class="wp-accent-button" @click="playAll">
            <span class="wp-icon">&#xE768;</span>播放全部
          </button>
          <button class="wp-standard-button" @click="shufflePlay">
            <span class="wp-icon">&#xE8B1;</span>随机播放
          </button>
        </div>
      </div>

      <SongTable
        :tracks="library.likedTracks"
        :show-source="true"
        @remove="(track) => library.toggleLike(track)"
      />
    </template>

    <EmptyState
      v-else
      icon="&#xE734;"
      title="还没有喜欢的音乐"
      description="浏览搜索结果或推荐，点击歌曲行的心形按钮即可收藏。"
    >
      <button class="wp-accent-button" @click="router.push('/')">去发现音乐</button>
    </EmptyState>
  </div>
</template>

<style scoped>
.header-cover {
  width: 96px;
  height: 96px;
  border-radius: var(--radius-lg);
  background: linear-gradient(135deg, var(--wp-accent), var(--wp-accent-active));
  color: var(--wp-text-on-accent);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 40px;
  flex: none;
  box-shadow: var(--shadow-card);
}

.header-meta {
  flex: 1;
  min-width: 0;
}

.header-sub {
  font-size: 13px;
  color: var(--wp-text-secondary);
  margin-top: var(--space-sm);
}

.header-actions {
  display: flex;
  gap: var(--space-md);
}
</style>
