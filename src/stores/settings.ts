/** 应用设置：主题、插件市场源等（持久化于 SQLite settings 表） */

import { defineStore } from "pinia";
import { ref } from "vue";
import { getSetting, setSetting } from "@/services/db";
import type { MarketSource } from "@/types/plugin";

export type ThemePreference = "system" | "light" | "dark";

/** 「来源偏好」默认值：空串表示不指定插件，由系统按当前逻辑自动决定 */
export type SourcePreference = string;

/** 播放源/歌词源搜索关键词的默认拼接格式 */
export const DEFAULT_SOURCE_SEARCH_FORMAT = "{歌曲名} - {歌手名}";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<ThemePreference>("system");
  const marketSources = ref<MarketSource[]>([]);
  /** GitHub 镜像源前缀（URL 拼接型，如 https://ghproxy.net/），空串表示直连 */
  const githubMirror = ref("");
  /** 是否启用系统媒体传输控件（SMTC，仅 Windows） */
  const smtcEnabled = ref(true);
  /** 歌曲信息源（插件 ID，空串为默认）：搜索结果聚合时置顶 */
  const trackSourceId = ref<SourcePreference>("");
  /** 播放源（插件 ID，空串为默认）：播放时按搜索格式在该插件重新搜曲并取其直链 */
  const playSourceId = ref<SourcePreference>("");
  /** 歌词源（插件 ID，空串为默认）：歌词按搜索格式在该插件重新搜曲后取词 */
  const lyricsSourceId = ref<SourcePreference>("");
  /** 头像源（插件 ID，空串为默认）：右上角头像优先从该插件获取 */
  const avatarSourceId = ref<SourcePreference>("");
  /** 播放源/歌词源共用的搜索关键词格式 */
  const sourceSearchFormat = ref(DEFAULT_SOURCE_SEARCH_FORMAT);
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
    trackSourceId.value = await getSetting<SourcePreference>("settings.trackSourceId", "");
    playSourceId.value = await getSetting<SourcePreference>("settings.playSourceId", "");
    lyricsSourceId.value = await getSetting<SourcePreference>("settings.lyricsSourceId", "");
    avatarSourceId.value = await getSetting<SourcePreference>("settings.avatarSourceId", "");
    sourceSearchFormat.value = await getSetting<string>(
      "settings.sourceSearchFormat",
      DEFAULT_SOURCE_SEARCH_FORMAT,
    );
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

  /** 通用来源偏好写入（四个来源选择框共用） */
  async function setSourcePreference(key: "trackSourceId" | "playSourceId" | "lyricsSourceId" | "avatarSourceId", value: SourcePreference): Promise<void> {
    const refs = { trackSourceId, playSourceId, lyricsSourceId, avatarSourceId };
    refs[key].value = value;
    await setSetting(`settings.${key}`, value);
  }

  async function setSourceSearchFormat(value: string): Promise<void> {
    sourceSearchFormat.value = value;
    await setSetting("settings.sourceSearchFormat", value);
  }

  return {
    theme,
    marketSources,
    githubMirror,
    smtcEnabled,
    trackSourceId,
    playSourceId,
    lyricsSourceId,
    avatarSourceId,
    sourceSearchFormat,
    loaded,
    load,
    setTheme,
    setMarketSources,
    setGithubMirror,
    setSmtcEnabled,
    setSourcePreference,
    setSourceSearchFormat,
  };
});
