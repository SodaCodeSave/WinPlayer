<script setup lang="ts">
import { ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ContentDialog } from "winuionweb";
import { useLibraryStore } from "@/stores/library";

const router = useRouter();
const route = useRoute();
const library = useLibraryStore();

const createDialogOpen = ref(false);
const newPlaylistName = ref("");
const newPlaylistDescription = ref("");

function isActive(name: string): boolean {
  return route.name === name;
}

async function confirmCreate() {
  const playlist = await library.createPlaylist(newPlaylistName.value, newPlaylistDescription.value);
  createDialogOpen.value = false;
  newPlaylistName.value = "";
  newPlaylistDescription.value = "";
  if (playlist) {
    void router.push({ name: "playlist", params: { id: playlist.id } });
  }
}
</script>

<template>
  <nav class="sidenav">
    <div class="sidenav-scroll">
      <ul class="nav-list">
        <li>
          <router-link :to="{ name: 'recommend' }" class="nav-item" :class="{ active: isActive('recommend') }">
            <span class="wp-icon nav-icon">&#xE80F;</span>
            <span class="nav-label">推荐</span>
          </router-link>
        </li>
        <li>
          <router-link :to="{ name: 'liked' }" class="nav-item" :class="{ active: isActive('liked') }">
            <span class="wp-icon nav-icon">&#xE734;</span>
            <span class="nav-label">我喜欢的音乐</span>
            <span v-if="library.liked.length" class="nav-badge">{{ library.liked.length }}</span>
          </router-link>
        </li>
      </ul>

      <div class="nav-section-header">
        <span class="nav-section-title">歌单</span>
        <button class="add-button" title="新建歌单" @click="createDialogOpen = true">
          <span class="wp-icon">&#xE710;</span>
        </button>
      </div>

      <ul class="nav-list">
        <li v-for="playlist in library.playlists" :key="playlist.id">
          <router-link
            :to="{ name: 'playlist', params: { id: playlist.id } }"
            class="nav-item"
            :class="{ active: route.name === 'playlist' && route.params.id === String(playlist.id) }"
          >
            <span class="wp-icon nav-icon">&#xE8FD;</span>
            <span class="nav-label ellipsis">{{ playlist.name }}</span>
            <span v-if="playlist.trackCount" class="nav-badge">{{ playlist.trackCount }}</span>
          </router-link>
        </li>
        <li v-if="!library.playlists.length" class="nav-empty">还没有歌单，点击 + 创建</li>
      </ul>
    </div>

    <div class="nav-footer">
      <ul class="nav-list">
        <li>
          <router-link :to="{ name: 'plugins' }" class="nav-item" :class="{ active: isActive('plugins') }">
            <span class="wp-icon nav-icon">&#xE943;</span>
            <span class="nav-label">插件管理</span>
          </router-link>
        </li>
        <li>
          <router-link :to="{ name: 'settings' }" class="nav-item" :class="{ active: isActive('settings') }">
            <span class="wp-icon nav-icon">&#xE713;</span>
            <span class="nav-label">设置</span>
          </router-link>
        </li>
      </ul>
    </div>

    <ContentDialog
      :IsOpen="createDialogOpen"
      Title="新建歌单"
      PrimaryButtonText="创建"
      CloseButtonText="取消"
      @PrimaryButtonClick="confirmCreate"
      @CloseButtonClick="createDialogOpen = false"
    >
      <div class="dialog-form">
        <input
          v-model="newPlaylistName"
          class="dialog-input"
          type="text"
          placeholder="歌单名称"
          maxlength="50"
        />
        <input
          v-model="newPlaylistDescription"
          class="dialog-input"
          type="text"
          placeholder="歌单简介（可选）"
          maxlength="200"
        />
      </div>
    </ContentDialog>
  </nav>
</template>

<style scoped>
.sidenav {
  display: flex;
  flex-direction: column;
  background: var(--wp-layer);
  border-right: 1px solid var(--wp-divider);
  padding: var(--space-md) var(--space-md) var(--space-md) var(--space-md);
}

.sidenav-scroll {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.nav-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  height: 36px;
  padding: 0 var(--space-lg);
  border-radius: var(--radius-sm);
  color: var(--wp-text-primary);
  font-size: 14px;
  text-decoration: none;
  position: relative;
  transition: background var(--duration-fast) var(--ease-standard);
}

.nav-item:hover {
  background: var(--wp-card-hover);
  text-decoration: none;
}

.nav-item:active {
  background: var(--wp-card-active);
}

.nav-item.active {
  background: var(--wp-card-active);
}

.nav-item.active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 16px;
  border-radius: 2px;
  background: var(--wp-accent);
}

.nav-icon {
  font-size: 16px;
  width: 20px;
  text-align: center;
  flex: none;
}

.nav-label {
  flex: 1;
  min-width: 0;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.nav-badge {
  font-size: 12px;
  color: var(--wp-text-tertiary);
  flex: none;
}

.nav-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-xl) var(--space-lg) var(--space-sm) var(--space-lg);
}

.nav-section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--wp-text-secondary);
}

.add-button {
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
  transition: background var(--duration-fast) var(--ease-standard);
}

.add-button:hover {
  background: var(--wp-card-hover);
  color: var(--wp-text-primary);
}

.nav-empty {
  padding: var(--space-md) var(--space-lg);
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.nav-footer {
  flex: none;
  padding-top: var(--space-md);
  border-top: 1px solid var(--wp-divider);
}

.dialog-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  padding: var(--space-md) 0;
}

.dialog-input {
  height: 32px;
  padding: 0 var(--space-md);
  border: 1px solid var(--wp-control-stroke-secondary);
  border-bottom-color: var(--wp-control-stroke-secondary);
  border-radius: var(--radius-sm);
  background: var(--wp-control-fill);
  color: var(--wp-text-primary);
  font-size: 14px;
  font-family: inherit;
  outline: none;
  user-select: text;
}

.dialog-input:focus {
  background: var(--wp-layer-solid);
  border-bottom: 2px solid var(--wp-accent);
}
</style>
