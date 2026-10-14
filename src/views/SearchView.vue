<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { usePluginsStore } from "@/stores/plugins";
import { usePlayerStore } from "@/stores/player";
import SongTable from "@/components/common/SongTable.vue";
import PlaylistCard from "@/components/common/PlaylistCard.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import { ProgressRing } from "winuionweb";
import type { SearchResult, UnifiedPlaylist, UnifiedTrack } from "@/types/plugin";

const route = useRoute();
const router = useRouter();
const plugins = usePluginsStore();
const player = usePlayerStore();

const keyword = ref("");
const activeTab = ref<"song" | "playlist">("song");
const loading = ref(false);
const searched = ref(false);

interface SourceResult {
  id: string;
  name: string;
  result: SearchResult | null;
  error?: string;
}

const sourceResults = ref<SourceResult[]>([]);

const allSongs = computed<UnifiedTrack[]>(() =>
  sourceResults.value.flatMap((source) => source.result?.songs ?? []),
);

const allPlaylists = computed<UnifiedPlaylist[]>(() =>
  sourceResults.value.flatMap((source) => source.result?.playlists ?? []),
);

async function search(keywordValue: string) {
  const trimmed = keywordValue.trim();
  if (!trimmed) return;
  searched.value = true;
  loading.value = true;
  sourceResults.value = [];
  try {
    const results = await plugins.searchAll(trimmed);
    sourceResults.value = results.map((item) => ({
      id: item.record.entry.id,
      name: item.record.entry.manifest.name,
      result: item.result,
      error: item.error,
    }));
  } finally {
    loading.value = false;
  }
}

function playAll() {
  if (allSongs.value.length) void player.playTracks(allSongs.value, 0);
}

watch(
  () => route.query.q,
  (value) => {
    if (typeof value === "string" && value) {
      keyword.value = value;
      void search(value);
    }
  },
  { immediate: true },
);

function submit() {
  const trimmed = keyword.value.trim();
  if (!trimmed) return;
  void router.replace({ path: "/search", query: { q: trimmed } });
}

onMounted(() => {
  if (!route.query.q && plugins.enabledRecords.length) {
    void plugins.getHotKeywords().then((words) => {
      if (words.length && !keyword.value) keyword.value = words[0] ?? "";
    });
  }
});
</script>

<template>
  <div class="wp-page">
    <div class="wp-page-header search-header">
      <h1 class="wp-page-title">搜索</h1>
      <div class="search-field">
        <span class="wp-icon search-icon">&#xE721;</span>
        <input
          v-model="keyword"
          class="search-input"
          type="text"
          placeholder="输入关键词，同时检索所有已启用的插件源"
          @keydown.enter="submit"
        />
      </div>
      <button class="wp-accent-button" :disabled="!keyword.trim() || loading" @click="submit">搜索</button>
    </div>

    <EmptyState
      v-if="!plugins.enabledRecords.length"
      icon="&#xE943;"
      title="没有可用的搜索来源"
      description="聚合搜索会同时检索所有启用了搜索能力的插件，请先在插件管理中安装并启用插件。"
    >
      <button class="wp-accent-button" @click="router.push('/plugins')">前往插件管理</button>
    </EmptyState>

    <template v-else>
      <div v-if="loading" class="loading-wrap">
        <ProgressRing :IsActive="true" :IsIndeterminate="true" :Width="36" :Height="36" />
        <span class="loading-text">正在搜索 {{ sourceResults.length ? "" : "所有启用的插件源" }}…</span>
      </div>

      <template v-else-if="searched">
        <div v-if="!allSongs.length && !allPlaylists.length" class="no-result">
          没有找到与「{{ route.query.q }}」相关的结果
        </div>

        <template v-else>
          <!-- 类型标签页 -->
          <div class="tabs">
            <button
              class="tab"
              :class="{ active: activeTab === 'song' }"
              @click="activeTab = 'song'"
            >
              单曲<span class="tab-count">{{ allSongs.length }}</span>
            </button>
            <button
              class="tab"
              :class="{ active: activeTab === 'playlist' }"
              @click="activeTab = 'playlist'"
            >
              歌单<span class="tab-count">{{ allPlaylists.length }}</span>
            </button>
            <button v-if="activeTab === 'song' && allSongs.length" class="tab-action" @click="playAll">
              <span class="wp-icon">&#xE768;</span>播放全部
            </button>
          </div>

          <!-- 单曲：按来源分组 -->
          <div v-if="activeTab === 'song'">
            <section v-for="source in sourceResults" :key="source.id" class="source-group">
              <h3 class="group-title">
                {{ source.name }}
                <span v-if="source.error" class="group-error">加载失败：{{ source.error }}</span>
                <span v-else-if="source.result?.songs?.length" class="group-count">
                  {{ source.result.songs.length }} 首
                </span>
              </h3>
              <SongTable
                v-if="source.result?.songs?.length"
                :tracks="source.result.songs"
                :show-source="false"
              />
            </section>
          </div>

          <!-- 歌单：卡片网格 -->
          <div v-else class="playlist-grid">
            <PlaylistCard v-for="playlist in allPlaylists" :key="`${playlist.sourceId}:${playlist.id}`" :playlist="playlist" />
            <div v-if="!allPlaylists.length" class="no-result">没有找到相关歌单</div>
          </div>
        </template>
      </template>
    </template>
  </div>
</template>

<style scoped>
.search-header {
  gap: var(--space-lg);
}

.search-field {
  flex: 1;
  display: flex;
  align-items: center;
  gap: var(--space-md);
  height: 36px;
  padding: 0 var(--space-lg);
  background: var(--wp-control-fill);
  border: 1px solid transparent;
  border-bottom-color: var(--wp-control-stroke-secondary);
  border-radius: var(--radius-sm);
  max-width: 560px;
}

.search-field:focus-within {
  background: var(--wp-layer-solid);
  border-bottom: 2px solid var(--wp-accent);
}

.search-icon {
  color: var(--wp-text-secondary);
  font-size: 14px;
}

.search-input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  color: var(--wp-text-primary);
  font-size: 14px;
  font-family: inherit;
  user-select: text;
}

.loading-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-lg);
  padding: 80px 0;
  color: var(--wp-text-secondary);
}

.loading-text,
.no-result {
  font-size: 13px;
}

.no-result {
  padding: var(--space-3xl) 0;
  color: var(--wp-text-tertiary);
  text-align: center;
}

.tabs {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  border-bottom: 1px solid var(--wp-divider);
  margin-bottom: var(--space-lg);
}

.tab {
  border: none;
  background: transparent;
  color: var(--wp-text-secondary);
  font-size: 14px;
  padding: var(--space-md) var(--space-lg);
  cursor: pointer;
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
}

.tab:hover {
  background: var(--wp-card-hover);
  color: var(--wp-text-primary);
}

.tab.active {
  color: var(--wp-text-primary);
  font-weight: 600;
}

.tab.active::after {
  content: "";
  position: absolute;
  left: var(--space-lg);
  right: var(--space-lg);
  bottom: 0;
  height: 3px;
  border-radius: 2px;
  background: var(--wp-accent);
}

.tab-count {
  font-size: 11px;
  color: var(--wp-text-tertiary);
  background: var(--wp-control-fill-secondary);
  border-radius: 8px;
  padding: 1px 6px;
}

.tab-action {
  margin-left: auto;
  border: none;
  background: transparent;
  color: var(--wp-accent);
  font-size: 13px;
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-lg);
  cursor: pointer;
  border-radius: var(--radius-sm);
}

.tab-action:hover {
  background: var(--wp-card-hover);
}

.source-group {
  margin-bottom: var(--space-2xl);
}

.group-title {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 var(--space-md) var(--space-sm);
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.group-count {
  font-size: 12px;
  font-weight: 400;
  color: var(--wp-text-tertiary);
}

.group-error {
  font-size: 12px;
  font-weight: 400;
  color: var(--wp-danger);
}

.playlist-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: var(--space-md);
}
</style>
