<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ContentDialog } from "winuionweb";
import { useLibraryStore } from "@/stores/library";
import { usePlayerStore } from "@/stores/player";
import SongTable from "@/components/common/SongTable.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import type { UnifiedTrack } from "@/types/plugin";

const route = useRoute();
const router = useRouter();
const library = useLibraryStore();
const player = usePlayerStore();

const playlistId = ref<number>(0);
const tracks = ref<UnifiedTrack[]>([]);
const loading = ref(false);
const renameOpen = ref(false);
const deleteOpen = ref(false);
const renameValue = ref("");

const playlist = () => library.playlists.find((item) => item.id === playlistId.value);

async function load(id: number) {
  loading.value = true;
  try {
    tracks.value = await library.getPlaylistTracks(id);
  } finally {
    loading.value = false;
  }
}

watch(
  () => route.params.id,
  (value) => {
    const id = Number(value);
    if (Number.isFinite(id) && id > 0) {
      playlistId.value = id;
      void load(id);
    } else {
      void router.replace("/");
    }
  },
  { immediate: true },
);

function playAll() {
  if (tracks.value.length) void player.playTracks(tracks.value, 0);
}

async function onRemove(track: UnifiedTrack) {
  await library.removeTrackFromPlaylist(playlistId.value, track);
  await load(playlistId.value);
}

function openRename() {
  renameValue.value = playlist()?.name ?? "";
  renameOpen.value = true;
}

async function confirmRename() {
  await library.renamePlaylist(playlistId.value, renameValue.value);
  renameOpen.value = false;
}

async function confirmDelete() {
  deleteOpen.value = false;
  await library.deletePlaylist(playlistId.value);
  void router.replace("/liked");
}

onMounted(() => {
  if (playlist()) renameValue.value = playlist()!.name;
});
</script>

<template>
  <div class="wp-page">
    <template v-if="playlist()">
      <div class="wp-page-header">
        <div class="header-cover">
          <img v-if="playlist()!.cover" :src="playlist()!.cover" alt="" class="cover-img" />
          <span v-else class="wp-icon">&#xE8FD;</span>
        </div>
        <div class="header-meta">
          <h1 class="wp-page-title">{{ playlist()!.name }}</h1>
          <div class="header-sub">
            {{ playlist()!.trackCount ?? tracks.length }} 首
            <span v-if="playlist()!.description"> · {{ playlist()!.description }}</span>
          </div>
          <div class="header-sub-actions">
            <button class="wp-standard-button small" @click="openRename">
              <span class="wp-icon">&#xE70F;</span>重命名
            </button>
            <button class="wp-standard-button small danger" @click="deleteOpen = true">
              <span class="wp-icon">&#xE74D;</span>删除歌单
            </button>
          </div>
        </div>
        <div class="header-actions">
          <button class="wp-accent-button" :disabled="!tracks.length" @click="playAll">
            <span class="wp-icon">&#xE768;</span>播放全部
          </button>
        </div>
      </div>

      <SongTable
        v-if="tracks.length"
        :tracks="tracks"
        :show-source="true"
        :removable="true"
        @remove="onRemove"
      />
      <EmptyState v-else-if="!loading" icon="&#xE8FD;" title="歌单还是空的" description="在搜索结果或喜欢的音乐里，把歌曲添加到这个歌单吧。" />
    </template>

    <ContentDialog
      :IsOpen="renameOpen"
      Title="重命名歌单"
      PrimaryButtonText="保存"
      CloseButtonText="取消"
      @PrimaryButtonClick="confirmRename"
      @CloseButtonClick="renameOpen = false"
    >
      <input v-model="renameValue" class="dialog-input" type="text" maxlength="50" />
    </ContentDialog>

    <ContentDialog
      :IsOpen="deleteOpen"
      Title="删除歌单"
      Content="删除后无法恢复，确定要删除这个歌单吗？"
      PrimaryButtonText="删除"
      CloseButtonText="取消"
      @PrimaryButtonClick="confirmDelete"
      @CloseButtonClick="deleteOpen = false"
    />
  </div>
</template>

<style scoped>
.header-cover {
  width: 96px;
  height: 96px;
  border-radius: var(--radius-lg);
  background: var(--wp-control-fill-secondary);
  color: var(--wp-text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 40px;
  flex: none;
  overflow: hidden;
  box-shadow: var(--shadow-card);
}

.cover-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
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

.header-sub-actions {
  display: flex;
  gap: var(--space-md);
  margin-top: var(--space-md);
}

.small {
  height: 26px;
  font-size: 12px;
  padding: 0 12px;
}

.header-actions {
  display: flex;
  gap: var(--space-md);
}

.dialog-input {
  width: 100%;
  height: 32px;
  padding: 0 var(--space-md);
  border: 1px solid var(--wp-control-stroke-secondary);
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
