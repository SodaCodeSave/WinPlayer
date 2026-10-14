/** 应用设置：主题、插件市场源等（持久化于 SQLite settings 表） */

import { defineStore } from "pinia";
import { ref } from "vue";
import { getSetting, setSetting } from "@/services/db";
import type { MarketSource } from "@/types/plugin";

export type ThemePreference = "system" | "light" | "dark";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<ThemePreference>("system");
  const marketSources = ref<MarketSource[]>([]);
  const loaded = ref(false);

  /** 将主题偏好应用到 document（winuionweb 通过 html[data-theme] 切换，缺省跟随系统） */
  function applyTheme(): void {
    if (theme.value === "system") {
      delete document.documentElement.dataset.theme;
    } else {
      document.documentElement.dataset.theme = theme.value;
    }
  }

  async function load(): Promise<void> {
    theme.value = await getSetting<ThemePreference>("settings.theme", "system");
    marketSources.value = await getSetting<MarketSource[]>("settings.marketSources", []);
    applyTheme();
    loaded.value = true;
  }

  async function setTheme(value: ThemePreference): Promise<void> {
    theme.value = value;
    applyTheme();
    await setSetting("settings.theme", value);
  }

  async function setMarketSources(value: MarketSource[]): Promise<void> {
    marketSources.value = value;
    await setSetting("settings.marketSources", value);
  }

  return { theme, marketSources, loaded, load, setTheme, setMarketSources };
});
