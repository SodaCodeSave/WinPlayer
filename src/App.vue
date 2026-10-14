<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { RouterView } from "vue-router";
import TitleBar from "@/components/layout/TitleBar.vue";
import SideNav from "@/components/layout/SideNav.vue";
import PlayerBar from "@/components/layout/PlayerBar.vue";
import ToastHost from "@/components/common/ToastHost.vue";
import { useSettingsStore } from "@/stores/settings";
import { usePluginsStore } from "@/stores/plugins";
import { useLibraryStore } from "@/stores/library";
import { usePlayerStore } from "@/stores/player";

const settings = useSettingsStore();
const plugins = usePluginsStore();
const library = useLibraryStore();
const player = usePlayerStore();

onMounted(async () => {
  await settings.load();
  await Promise.all([library.loadAll(), player.restore()]);
  await plugins.loadInstalled();
});

onUnmounted(() => {
  plugins.disposeAll();
});
</script>

<template>
  <div class="app-shell">
    <TitleBar class="app-titlebar" />
    <div class="app-body">
      <SideNav class="app-sidenav" />
      <main class="app-content">
        <router-view v-slot="{ Component }">
          <transition name="wp-route" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>
    <PlayerBar class="app-playerbar" />
    <ToastHost />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--wp-base);
  color: var(--wp-text-primary);
}

.app-titlebar {
  height: var(--titlebar-height);
  flex: none;
  z-index: 40;
}

.app-body {
  flex: 1;
  display: flex;
  min-height: 0;
}

.app-sidenav {
  width: var(--sidenav-width);
  flex: none;
}

.app-content {
  flex: 1;
  min-width: 0;
  position: relative;
}

.app-playerbar {
  height: var(--playerbar-height);
  flex: none;
  z-index: 30;
}
</style>
