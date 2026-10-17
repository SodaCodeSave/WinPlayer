/** 应用设置：主题、插件市场源等（持久化于 SQLite settings 表） */

import { defineStore } from "pinia";
import { ref } from "vue";
import { getSetting, setSetting } from "@/services/db";
import { getAudioEngine } from "@/services/audio";
import {
  EQ_CUSTOM_PRESET_ID,
  EQ_PRESETS,
  FLAT_GAINS,
  findEqPreset,
} from "@/constants/eqPresets";
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
  /** 均衡器开关 */
  const eqEnabled = ref(false);
  /** 均衡器预设 ID（手动调整滑杆后为 custom） */
  const eqPreset = ref("flat");
  /** 均衡器 10 段增益（dB） */
  const eqGains = ref<number[]>([...FLAT_GAINS]);
  /** 全局字体（CSS font-family 值，空串 = 使用默认字体） */
  const fontFamily = ref("");
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

  /** 将自定义字体覆盖到全局字体变量；空值移除覆盖回退到 tokens 默认 */
  function applyFont(): void {
    const root = document.documentElement;
    const value = fontFamily.value.trim();
    if (value) {
      root.style.setProperty("--font-family-display", value);
    } else {
      root.style.removeProperty("--font-family-display");
    }
  }

  // 首次创建即应用（设置尚未从数据库加载时的默认值）
  applyTheme();
  applyFont();

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
    eqEnabled.value = await getSetting<boolean>("settings.eqEnabled", false);
    eqPreset.value = await getSetting<string>("settings.eqPreset", "flat");
    eqGains.value = await getSetting<number[]>("settings.eqGains", [...FLAT_GAINS]);
    fontFamily.value = await getSetting<string>("settings.fontFamily", "");
    applyTheme();
    applyFont();
    // 启动时把均衡器配置应用到原生引擎
    applyEq();
    loaded.value = true;
  }

  async function setTheme(value: ThemePreference): Promise<void> {
    theme.value = value;
    applyTheme();
    await setSetting("settings.theme", value);
  }

  async function setFontFamily(value: string): Promise<void> {
    fontFamily.value = value;
    applyFont();
    await setSetting("settings.fontFamily", value);
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

  /* ------------------------------ 均衡器 ------------------------------ */

  function applyEq(): void {
    getAudioEngine().setEq(eqEnabled.value, eqGains.value);
  }

  async function setEqEnabled(value: boolean): Promise<void> {
    eqEnabled.value = value;
    applyEq();
    await setSetting("settings.eqEnabled", value);
  }

  /** 切换预设：自动套用预设增益；custom 仅作为 UI 标识传入 */
  async function setEqPreset(id: string): Promise<void> {
    eqPreset.value = id;
    const preset = findEqPreset(id);
    if (preset) {
      eqGains.value = [...preset.gains];
      applyEq();
      await setSetting("settings.eqGains", eqGains.value);
    }
    await setSetting("settings.eqPreset", id);
  }

  /** 滑杆拖动期间的高频写入防抖计时器 */
  let eqGainsPersistTimer: ReturnType<typeof setTimeout> | undefined;

  /** 手动调整某段增益：预设转为「自定义」（增益持久化防抖，拖动结束才落库） */
  async function setEqGains(gains: number[]): Promise<void> {
    eqGains.value = gains;
    if (eqPreset.value !== EQ_CUSTOM_PRESET_ID) {
      eqPreset.value = EQ_CUSTOM_PRESET_ID;
      await setSetting("settings.eqPreset", EQ_CUSTOM_PRESET_ID);
    }
    applyEq();
    clearTimeout(eqGainsPersistTimer);
    eqGainsPersistTimer = setTimeout(() => {
      void setSetting("settings.eqGains", gains);
    }, 400);
  }

  /** 重置为平直增益（预设回到「平坦」） */
  async function resetEq(): Promise<void> {
    await setEqPreset(EQ_PRESETS[0].id);
  }

  return {
    theme,
    fontFamily,
    marketSources,
    githubMirror,
    smtcEnabled,
    trackSourceId,
    playSourceId,
    lyricsSourceId,
    avatarSourceId,
    sourceSearchFormat,
    eqEnabled,
    eqPreset,
    eqGains,
    loaded,
    load,
    setTheme,
    setFontFamily,
    setMarketSources,
    setGithubMirror,
    setSmtcEnabled,
    setSourcePreference,
    setSourceSearchFormat,
    setEqEnabled,
    setEqPreset,
    setEqGains,
    resetEq,
  };
});
