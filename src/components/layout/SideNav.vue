<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ContentDialog, TextBox } from "winuionweb";
import { useLibraryStore } from "@/stores/library";

const router = useRouter();
const route = useRoute();
const library = useLibraryStore();

/** ContentDialog 通过 ShowAsync() 打开（库没有 IsOpen prop） */
interface DialogRef {
  ShowAsync: () => Promise<string>;
}
const createDialogRef = ref<DialogRef | null>(null);
const newPlaylistName = ref("");
const newPlaylistDescription = ref("");

async function openCreateDialog() {
  newPlaylistName.value = "";
  newPlaylistDescription.value = "";
  const result = await createDialogRef.value?.ShowAsync();
  if (result !== "Primary") return;
  const playlist = await library.createPlaylist(newPlaylistName.value, newPlaylistDescription.value);
  if (playlist) {
    void router.push({ name: "playlist", params: { id: playlist.id } });
  }
}

/* 滑动选中药丸（WinUI NavigationView 风格） */
const scrollPill = ref({ top: 0, visible: false });
const footerPill = ref({ top: 0, visible: false });
const pillsReady = ref(false);

function updatePill(container: Element | null, pill: typeof scrollPill): void {
  if (!container) return;
  const active = container.querySelector<HTMLElement>(".nav-item.active");
  if (!active) {
    pill.value.visible = false;
    return;
  }
  pill.value = { top: active.offsetTop, visible: true };
}

function updatePills(): void {
  updatePill(document.querySelector(".sidenav-scroll"), scrollPill);
  updatePill(document.querySelector(".nav-footer"), footerPill);
}

function pillStyle(pill: { top: number; visible: boolean }) {
  return {
    top: `${pill.top}px`,
    opacity: pill.visible ? "1" : "0",
    transition: pillsReady.value ? undefined : "none",
  };
}

/** 解除初始定位的 transition:none。
 * rAF 在页面不可见（窗口最小化/被完全遮挡）时不会触发，会导致
 * transition:none 永久残留、导航时药丸瞬移，故加 setTimeout 兜底。 */
function markPillsReady(): void {
  if (pillsReady.value) return;
  pillsReady.value = true;
}

onMounted(() => {
  void nextTick(() => {
    updatePills();
    requestAnimationFrame(markPillsReady);
    window.setTimeout(markPillsReady, 320);
  });
});

watch(
  () => [route.fullPath, library.playlists.length],
  async () => {
    await nextTick();
    updatePills();
  },
);

function isActive(name: string): boolean {
  return route.name === name;
}
</script>

<template>
  <nav class="sidenav">
    <div class="sidenav-scroll">
      <span class="nav-pill" :style="pillStyle(scrollPill)" aria-hidden="true" />
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
        <button class="add-button" title="新建歌单" @click="openCreateDialog">
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
      <span class="nav-pill" :style="pillStyle(footerPill)" aria-hidden="true" />
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
      ref="createDialogRef"
      Title="新建歌单"
      PrimaryButtonText="创建"
      CloseButtonText="取消"
    >
      <div class="dialog-form">
        <TextBox
          :Text="newPlaylistName"
          PlaceholderText="歌单名称"
          :MaxLength="50"
          @update:Text="newPlaylistName = String($event ?? '')"
        />
        <TextBox
          :Text="newPlaylistDescription"
          PlaceholderText="歌单简介（可选）"
          :MaxLength="200"
          @update:Text="newPlaylistDescription = String($event ?? '')"
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
  position: relative;
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
  z-index: 1;
  transition: color var(--duration-fast) var(--ease-standard);
}

.nav-item:hover {
  color: var(--wp-text-primary);
  text-decoration: none;
}

/* 选中药丸：在两个容器（滚动区/底栏）内滑动切换，由 JS 定位 top */
.nav-pill {
  position: absolute;
  left: 0;
  right: 0;
  height: 36px;
  border-radius: var(--radius-sm);
  background: var(--wp-card-active);
  z-index: 0;
  pointer-events: none;
  transition:
    top 300ms cubic-bezier(0.1, 0.9, 0.2, 1),
    opacity 200ms var(--ease-standard);
}

.nav-footer {
  flex: none;
  position: relative;
  padding-top: var(--space-md);
  border-top: 1px solid var(--wp-divider);
}

/* 左侧强调色小条（静态，不参与动画；动画只有选中药丸滑动） */
.nav-item.active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 50%;
  width: 3px;
  height: 16px;
  border-radius: 2px;
  background: var(--wp-accent);
  transform: translateY(-50%);
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

.dialog-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  padding: var(--space-md) 0;
}
</style>
