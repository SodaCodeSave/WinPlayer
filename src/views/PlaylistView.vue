<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ContentDialog, TextBox } from "winuionweb";
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
const renameValue = ref("");

/** ContentDialog 通过 ShowAsync() 打开（库没有 IsOpen prop） */
interface DialogRef {
  ShowAsync: () => Promise<string>;
}
const renameDialogRef = ref<DialogRef | null>(null);
const deleteDialogRef = ref<DialogRef | null>(null);

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

async function openRename() {
  renameValue.value = playlist()?.name ?? "";
  const result = await renameDialogRef.value?.ShowAsync();
  if (result !== "Primary") return;
  await library.renamePlaylist(playlistId.value, renameValue.value);
}

async function openDelete() {
  const result = await deleteDialogRef.value?.ShowAsync();
  if (result !== "Primary") return;
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
            <button class="wp-standard-button small danger" @click="openDelete">
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
      ref="renameDialogRef"
      Title="重命名歌单"
      PrimaryButtonText="保存"
      CloseButtonText="取消"
    >
      <div class="dialog-form">
        <TextBox
          :Text="renameValue"
          PlaceholderText="歌单名称"
          :MaxLength="50"
          @update:Text="renameValue = String($event ?? '')"
        />
      </div>
    </ContentDialog>

    <ContentDialog
      ref="deleteDialogRef"
      Title="删除歌单"
      Content="删除后无法恢复，确定要删除这个歌单吗？"
      PrimaryButtonText="删除"
      CloseButtonText="取消"
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

.dialog-form {
  padding: var(--space-md) 0;
}
</style>
