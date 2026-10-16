/** 应用设置：主题、插件市场源等（持久化于 SQLite settings 表） */

import { defineStore } from "pinia";
import { ref } from "vue";
import { getSetting, setSetting } from "@/services/db";
import type { MarketSource } from "@/types/plugin";

export type ThemePreference = "system" | "light" | "dark";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<ThemePreference>("system");
  const marketSources = ref<MarketSource[]>([]);
  /** GitHub 镜像源前缀（URL 拼接型，如 https://ghproxy.net/），空串表示直连 */
  const githubMirror = ref("");
  /** 是否启用系统媒体传输控件（SMTC，仅 Windows） */
  const smtcEnabled = ref(true);
  const loaded = ref(false);

  /** 系统配色查询：winuionweb 的暗色控件变量只认 html 上的 theme-dark 类，
   * 仅 data-theme 属性不会切换其输入框/下拉框配色，故类名必须同步。
   * 跟随系统时也解析出实际配色写入属性与类，保证控件库与自身 tokens 一致。 */
  const darkScheme = window.matchMedia("(prefers-color-scheme: dark)");

  function applyTheme(): void {
    const root = document.documentElement;
    const dark = theme.value === "system" ? darkScheme.matches : theme.value === "dark";
    root.dataset.theme = dark ? "dark" : "light";
    root.classList.toggle("theme-dark", dark);
    root.classList.toggle("theme-light", !dark);
  }

  // 跟随系统时，系统配色变化后重新应用
  darkScheme.addEventListener("change", () => {
    if (theme.value === "system") applyTheme();
  });

  // 首次创建即应用（设置尚未从数据库加载时的默认值）
  applyTheme();

  async function load(): Promise<void> {
    theme.value = await getSetting<ThemePreference>("settings.theme", "system");
    marketSources.value = await getSetting<MarketSource[]>("settings.marketSources", []);
    githubMirror.value = await getSetting<string>("settings.githubMirror", "");
    smtcEnabled.value = await getSetting("settings.smtcEnabled", true);
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

  async function setGithubMirror(value: string): Promise<void> {
    githubMirror.value = value.trim().replace(/\/+$/, "");
    await setSetting("settings.githubMirror", githubMirror.value);
  }

  async function setSmtcEnabled(value: boolean): Promise<void> {
    smtcEnabled.value = value;
    await setSetting("settings.smtcEnabled", value);
  }

  return {
    theme,
    marketSources,
    githubMirror,
    smtcEnabled,
    loaded,
    load,
    setTheme,
    setMarketSources,
    setGithubMirror,
    setSmtcEnabled,
  };
});
