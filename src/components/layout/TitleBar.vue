<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { PersonPicture } from "winuionweb";
import { pluginDisplayName, usePluginsStore, type AppAvatar } from "@/stores/plugins";

const router = useRouter();
const appWindow = getCurrentWindow();
const plugins = usePluginsStore();

const query = ref("");
const maximized = ref(false);

/** 插件提供的头像（能力：avatar），为空时回退默认占位头像 */
const avatar = ref<AppAvatar | null>(null);

const avatarUrl = computed(() => avatar.value?.image ?? "");
const avatarTitle = computed(() => {
  if (!avatar.value) return "账户";
  const name = pluginDisplayName(plugins.records, avatar.value.sourceId);
  const label = avatar.value.label?.trim();
  return [label, name].filter(Boolean).join(" · ") || "账户";
});

async function loadAvatar() {
  try {
    avatar.value = await plugins.getAvatar();
  } catch {
    avatar.value = null;
  }
}

/** 插件安装/启停/配置变化后重新取头像（配置变更会重建沙箱） */
watch(
  () => plugins.records,
  () => void loadAvatar(),
  { deep: true },
);

let unlistenResize: UnlistenFn | null = null;

async function toggleMaximize() {
  await appWindow.toggleMaximize();
}

onMounted(async () => {
  void loadAvatar();
  unlistenResize = await appWindow.onResized(() => {
    void appWindow.isMaximized().then((value) => {
      maximized.value = value;
    });
  });
  maximized.value = await appWindow.isMaximized();
});

onUnmounted(() => {
  unlistenResize?.();
});

function submitSearch() {
  const keyword = query.value.trim();
  if (!keyword) return;
  void router.push({ path: "/search", query: { q: keyword } });
}
</script>

<template>
  <header class="titlebar">
    <!-- 拖拽区域 -->
    <div class="drag-region" data-tauri-drag-region />

    <div class="titlebar-left" data-tauri-drag-region>
      <div class="logo">
        <span class="logo-mark">
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
            <path
              fill="currentColor"
              d="M13.5 2.2a.75.75 0 0 0-.9-.72L6.6 2.9a.75.75 0 0 0-.6.74v6.28A2.75 2.75 0 1 0 7.5 12.6V7.3l5-1.24v3.36A2.75 2.75 0 1 0 13.5 12V2.2z"
            />
          </svg>
        </span>
        <span class="logo-text">WinPlayer</span>
      </div>
    </div>

    <div class="titlebar-center">
      <div class="search-box">
        <span class="wp-icon search-icon">&#xE721;</span>
        <input
          v-model="query"
          class="search-input"
          type="text"
          placeholder="搜索音乐、歌单"
          @keydown.enter="submitSearch"
        />
      </div>
    </div>

    <div class="titlebar-right">
      <div class="avatar-wrap">
        <button class="avatar-button" :title="avatarTitle">
          <img v-if="avatarUrl" class="avatar-image" :src="avatarUrl" alt="" draggable="false" />
          <PersonPicture v-else DisplayName="音" :Width="28" :Height="28" FontSize="12" />
        </button>
      </div>

      <div class="caption-buttons">
        <button class="caption-button" title="最小化" @click="appWindow.minimize()">
          <span class="wp-icon">&#xE921;</span>
        </button>
        <button class="caption-button" :title="maximized ? '还原' : '最大化'" @click="toggleMaximize">
          <span class="wp-icon">{{ maximized ? "&#xE923;" : "&#xE922;" }}</span>
        </button>
        <button class="caption-button close" title="关闭" @click="appWindow.close()">
          <span class="wp-icon">&#xE8BB;</span>
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: relative;
  display: flex;
  align-items: center;
  background: var(--wp-base);
  border-bottom: 1px solid var(--wp-divider);
}

.drag-region {
  position: absolute;
  inset: 0;
  z-index: 0;
}

.titlebar-left {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  padding-left: var(--space-xl);
  min-width: 200px;
}

.logo {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.logo-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  background: var(--wp-accent);
  color: var(--wp-text-on-accent);
}

.logo-text {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.2px;
}

.titlebar-center {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1;
  width: min(520px, 42%);
}

.search-box {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  height: 32px;
  padding: 0 var(--space-md);
  background: var(--wp-control-fill);
  border: 1px solid transparent;
  border-bottom-color: var(--wp-control-stroke-secondary);
  border-radius: var(--radius-sm);
  transition: background var(--duration-fast) var(--ease-standard);
}

.search-box:hover {
  background: var(--wp-card-hover);
}

.search-box:focus-within {
  background: var(--wp-layer-solid);
  border-bottom: 2px solid var(--wp-accent);
}

.search-icon {
  font-size: 13px;
  color: var(--wp-text-secondary);
}

.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  color: var(--wp-text-primary);
  font-size: 13px;
  font-family: inherit;
  user-select: text;
}

.search-input::placeholder {
  color: var(--wp-text-secondary);
}

.titlebar-right {
  position: relative;
  z-index: 1;
  margin-left: auto;
  display: flex;
  align-items: stretch;
  height: 100%;
}

.avatar-wrap {
  display: flex;
  align-items: center;
  padding: 0 var(--space-md);
}

.avatar-button {
  border: none;
  background: transparent;
  padding: 0;
  cursor: pointer;
  border-radius: 50%;
}

.avatar-image {
  display: block;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  object-fit: cover;
  background: var(--wp-control-fill);
}

.caption-buttons {
  display: flex;
}

.caption-button {
  width: 46px;
  border: none;
  background: transparent;
  color: var(--wp-text-primary);
  font-size: 11px;
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-standard);
}

.caption-button:hover {
  background: var(--wp-card-hover);
}

.caption-button:active {
  background: var(--wp-card-active);
}

.caption-button.close:hover {
  background: #c42b1c;
  color: #ffffff;
}

.caption-button.close:active {
  background: #b22a1d;
}
</style>
