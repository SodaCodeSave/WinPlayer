/** 插件管理：安装/卸载/启停/配置，以及跨插件的聚合调用 */

import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { PluginHost } from "@/plugin-runtime/host";
import { getDb } from "@/services/db";
import { httpFetchJson } from "@/services/proxy";
import { toast } from "@/services/toast";
import type {
  LoginPollResult,
  LoginQr,
  LoginState,
  LyricsResult,
  MarketEntry,
  MarketIndex,
  MarketSource,
  PlaylistDetail,
  PluginCapability,
  PluginEntry,
  PluginManifest,
  RecommendResult,
  SearchResult,
  SongUrlResult,
  UnifiedPlaylist,
  UnifiedTrack,
} from "@/types/plugin";

export interface PluginRecord {
  entry: PluginEntry;
  enabled: boolean;
  settings: Record<string, unknown>;
  /** 最近一次运行时错误（用于插件管理页展示） */
  error?: string;
}

export interface AggregatedSourceResult<T> {
  record: PluginRecord;
  result: T | null;
  error?: string;
}

/** 从 manifest 设置定义生成默认值 */
export function defaultSettingsFromManifest(manifest: PluginManifest): Record<string, unknown> {
  const values: Record<string, unknown> = {};
  for (const item of manifest.settings ?? []) {
    values[item.key] = item.default ?? (item.type === "switch" ? false : "");
  }
  return values;
}

/**
 * 登录凭证存放的设置项 key。
 *
 * 登录凭证属于「插件私有配置」，由宿主统一持久化到该 key，而不是让每个插件
 * 自己想办法落盘——这样设置表单天然能展示/清除凭证，退出登录也只是清空它。
 */
export function credentialKeyOf(manifest: PluginManifest): string {
  return manifest.login?.credentialKey || "credential";
}

export const usePluginsStore = defineStore("plugins", () => {
  const records = ref<PluginRecord[]>([]);
  const loading = ref(false);

  /** 运行时宿主（非响应式） */
  const hosts = new Map<string, PluginHost>();

  const enabledRecords = computed(() => records.value.filter((r) => r.enabled));

  function hasCapability(record: PluginRecord, capability: PluginCapability): boolean {
    return record.entry.manifest.capabilities?.includes(capability) ?? false;
  }

  async function persistState(record: PluginRecord): Promise<void> {
    const db = await getDb();
    await db.execute(
      `insert into plugin_state (plugin_id, enabled, settings) values ($1, $2, $3)
       on conflict(plugin_id) do update set enabled = $2, settings = $3`,
      [record.entry.id, record.enabled ? 1 : 0, JSON.stringify(record.settings ?? {})],
    );
  }

  /** 加载已安装插件列表（磁盘 + 启用状态/配置） */
  async function loadInstalled(): Promise<void> {
    loading.value = true;
    try {
      const entries = await invoke<PluginEntry[]>("plugin_list");
      const db = await getDb();
      const states = await db.select<Array<{ plugin_id: string; enabled: number; settings: string }>>(
        "select plugin_id, enabled, settings from plugin_state",
      );
      const stateMap = new Map(states.map((row) => [row.plugin_id, row]));

      records.value = entries.map((entry) => {
        const row = stateMap.get(entry.id);
        let settings: Record<string, unknown> = defaultSettingsFromManifest(entry.manifest);
        if (row?.settings) {
          try {
            settings = { ...settings, ...(JSON.parse(row.settings) as Record<string, unknown>) };
          } catch {
            /* 配置损坏时回退默认值 */
          }
        }
        return {
          entry,
          enabled: row ? row.enabled === 1 : true,
          settings,
        };
      });

      // 清理已卸载插件的运行时
      for (const [id, host] of hosts) {
        if (!records.value.some((r) => r.entry.id === id)) {
          host.dispose();
          hosts.delete(id);
        }
      }
    } catch (error) {
      console.error("加载插件列表失败", error);
      toast(`加载插件列表失败: ${error}`, "error");
    } finally {
      loading.value = false;
    }
  }

  /** 获取插件运行时（懒加载，首次调用时启动沙箱） */
  async function getHost(id: string): Promise<PluginHost> {
    const record = records.value.find((r) => r.entry.id === id);
    if (!record) throw new Error(`插件不存在: ${id}`);
    if (!record.enabled) throw new Error(`插件已停用: ${record.entry.manifest.name}`);

    const existing = hosts.get(id);
    if (existing) return existing;

    const code = await invoke<string>("plugin_read_code", { id });
    const host = new PluginHost(record.entry, code, record.settings);
    hosts.set(id, host);
    try {
      await host.ready;
      record.error = undefined;
    } catch (error) {
      hosts.delete(id);
      host.dispose();
      record.error = error instanceof Error ? error.message : String(error);
      throw error;
    }
    return host;
  }

  async function togglePlugin(id: string, enabled: boolean): Promise<void> {
    const record = records.value.find((r) => r.entry.id === id);
    if (!record) return;
    record.enabled = enabled;
    await persistState(record);
    if (!enabled) {
      hosts.get(id)?.dispose();
      hosts.delete(id);
    }
  }

  /** 保存插件配置并重建运行时 */
  async function updateSettings(id: string, values: Record<string, unknown>): Promise<void> {
    const record = records.value.find((r) => r.entry.id === id);
    if (!record) return;
    record.settings = { ...defaultSettingsFromManifest(record.entry.manifest), ...values };
    await persistState(record);
    hosts.get(id)?.dispose();
    hosts.delete(id);
  }

  /** 从本地文件/目录/zip 安装 */
  async function installFromDialog(): Promise<boolean> {
    const path = await open({
      multiple: false,
      directory: false,
      title: "选择插件包（.zip / .json）",
      filters: [{ name: "WinPlayer 插件", extensions: ["zip", "json"] }],
    });
    if (!path || Array.isArray(path)) return false;
    try {
      const entry = await invoke<PluginEntry>("plugin_install_from_path", { src: path });
      await loadInstalled();
      toast(`插件「${entry.manifest.name}」安装成功`, "success");
      return true;
    } catch (error) {
      toast(`安装失败: ${error}`, "error");
      return false;
    }
  }

  /** 从 URL 安装 */
  async function installFromUrl(url: string): Promise<boolean> {
    try {
      const entry = await invoke<PluginEntry>("plugin_install_from_url", { url });
      await loadInstalled();
      toast(`插件「${entry.manifest.name}」安装成功`, "success");
      return true;
    } catch (error) {
      toast(`安装失败: ${error}`, "error");
      return false;
    }
  }

  async function uninstall(id: string): Promise<void> {
    const record = records.value.find((r) => r.entry.id === id);
    hosts.get(id)?.dispose();
    hosts.delete(id);
    try {
      await invoke("plugin_remove", { id });
      await loadInstalled();
      toast(`插件「${record?.entry.manifest.name ?? id}」已卸载`, "success");
    } catch (error) {
      toast(`卸载失败: ${error}`, "error");
    }
  }

  /** 卸载全部运行时（应用退出时调用） */
  function disposeAll(): void {
    for (const host of hosts.values()) host.dispose();
    hosts.clear();
  }

  /* ------------------------------ 聚合调用 ------------------------------ */

  /** 聚合搜索：并发调用所有启用且支持搜索的插件 */
  async function searchAll(keyword: string): Promise<AggregatedSourceResult<SearchResult>[]> {
    const targets = enabledRecords.value.filter((r) => hasCapability(r, "search"));
    return Promise.all(
      targets.map(async (record): Promise<AggregatedSourceResult<SearchResult>> => {
        try {
          const host = await getHost(record.entry.id);
          const result = await host.invoke<SearchResult>("search", { keyword, page: 1 });
          return { record, result: normalizeSearchResult(result, record.entry.id) };
        } catch (error) {
          return { record, result: null, error: error instanceof Error ? error.message : String(error) };
        }
      }),
    );
  }

  /** 聚合推荐 */
  async function recommendAll(): Promise<AggregatedSourceResult<RecommendResult>[]> {
    const targets = enabledRecords.value.filter((r) => hasCapability(r, "recommend"));
    return Promise.all(
      targets.map(async (record): Promise<AggregatedSourceResult<RecommendResult>> => {
        try {
          const host = await getHost(record.entry.id);
          const result = await host.invoke<RecommendResult>("getRecommendations", {});
          return { record, result: normalizeRecommendResult(result, record.entry.id) };
        } catch (error) {
          return { record, result: null, error: error instanceof Error ? error.message : String(error) };
        }
      }),
    );
  }

  /** 获取某首歌的播放直链 */
  async function getSongUrl(track: UnifiedTrack): Promise<SongUrlResult> {
    if (!track.sourceId) throw new Error("歌曲缺少来源信息");
    const host = await getHost(track.sourceId);
    const result = await host.invoke<SongUrlResult>("getSongUrl", { id: track.id });
    if (!result?.url) throw new Error("来源未返回可用的播放地址");
    return result;
  }

  /** 获取歌词（失败返回 null，不打断播放） */
  async function getLyrics(track: UnifiedTrack): Promise<LyricsResult | null> {
    if (!track.sourceId) return null;
    try {
      const host = await getHost(track.sourceId);
      return await host.invoke<LyricsResult>("getLyrics", { id: track.id });
    } catch {
      return null;
    }
  }

  /** 获取插件来源的歌单详情 */
  async function getPlaylistDetail(sourceId: string, playlistId: string): Promise<PlaylistDetail | null> {
    const host = await getHost(sourceId);
    const detail = await host.invoke<PlaylistDetail>("getPlaylist", { id: playlistId });
    if (detail) {
      detail.sourceId = sourceId;
      detail.tracks = (detail.tracks ?? []).map((t) => ({ ...t, sourceId }));
    }
    return detail;
  }

  /** 获取热搜词（聚合所有支持的插件） */
  async function getHotKeywords(): Promise<string[]> {
    const targets = enabledRecords.value.filter((r) => hasCapability(r, "hotKeywords"));
    const results = await Promise.allSettled(
      targets.map(async (record) => {
        const host = await getHost(record.entry.id);
        return host.invoke<string[]>("getHotKeywords", {});
      }),
    );
    const words = new Set<string>();
    for (const result of results) {
      if (result.status === "fulfilled" && Array.isArray(result.value)) {
        for (const word of result.value) if (word) words.add(String(word));
      }
    }
    return [...words];
  }

  /* ------------------------------ 登录（能力：login） ------------------------------ */

  /** 生成登录二维码；插件未实现登录能力时返回 null */
  async function getLoginQr(sourceId: string): Promise<LoginQr | null> {
    const record = getRecordOrThrow(sourceId);
    if (!hasCapability(record, "login")) return null;
    const host = await getHost(sourceId);
    return await host.invoke<LoginQr>("getLoginQr", { fresh: true });
  }

  /**
   * 轮询扫码状态。
   *
   * 返回 success 时会把凭证写回插件配置的 credentialKey 设置项——该写入会重建
   * Worker，因此调用方在拿到 success 后必须停止轮询，否则下一次 invoke 会抛
   * “插件已卸载”。
   */
  async function pollLogin(sourceId: string, key?: string): Promise<LoginPollResult | null> {
    const record = getRecordOrThrow(sourceId);
    if (!hasCapability(record, "login")) return null;
    const host = await getHost(sourceId);
    const result = await host.invoke<LoginPollResult>("pollLogin", key ? { key } : {});

    if (result?.status !== "success") return result;
    if (!result.credential) {
      // 成功但没拿到凭证，宁可报错也不能让 UI 显示“已登录”却查不到任何数据
      return { status: "error", message: "插件未返回登录凭证" };
    }
    const credentialKey = credentialKeyOf(record.entry.manifest);
    await updateSettings(sourceId, { ...record.settings, [credentialKey]: result.credential });
    return result;
  }

  /** 查询当前登录态（插件未实现时返回 null） */
  async function getLoginState(sourceId: string): Promise<LoginState | null> {
    try {
      const host = await getHost(sourceId);
      return await host.invoke<LoginState>("getLoginState", {});
    } catch {
      return null;
    }
  }

  /** 退出登录：清空凭证设置项（同样会重建 Worker） */
  async function logout(sourceId: string): Promise<void> {
    const record = getRecordOrThrow(sourceId);
    if (!hasCapability(record, "login")) return;
    try {
      const host = await getHost(sourceId);
      await host.invoke("logout", {});
    } catch {
      /* 插件侧清理失败不影响凭证清除 */
    }
    const credentialKey = credentialKeyOf(record.entry.manifest);
    await updateSettings(sourceId, { ...record.settings, [credentialKey]: "" });
  }

  /** 按 id 找到插件记录，找不到时抛出可读错误 */
  function getRecordOrThrow(sourceId: string): PluginRecord {
    const record = records.value.find((r) => r.entry.id === sourceId);
    if (!record) throw new Error(`插件 ${sourceId} 未安装`);
    return record;
  }

  /* ------------------------------ 插件市场 ------------------------------ */

  /** 拉取所有市场源的索引，合并为可安装条目列表 */
  async function fetchMarketEntries(sources: MarketSource[]): Promise<MarketEntry[]> {
    const results = await Promise.allSettled(
      sources.map(async (source) => {
        const index = await httpFetchJson<MarketIndex>(source.url, 15_000);
        return (index.plugins ?? []).map((entry) => ({ ...entry, sourceName: source.name }));
      }),
    );
    const entries: MarketEntry[] = [];
    for (const result of results) {
      if (result.status === "fulfilled") entries.push(...result.value);
    }
    return entries;
  }

  return {
    records,
    loading,
    enabledRecords,
    hasCapability,
    loadInstalled,
    getHost,
    togglePlugin,
    updateSettings,
    installFromDialog,
    installFromUrl,
    uninstall,
    disposeAll,
    searchAll,
    recommendAll,
    getSongUrl,
    getLyrics,
    getPlaylistDetail,
    getHotKeywords,
    getLoginQr,
    pollLogin,
    getLoginState,
    logout,
    fetchMarketEntries,
  };
});

/* --------------------------- 结果归一化辅助 --------------------------- */

function normalizeSearchResult(result: SearchResult | null, sourceId: string): SearchResult {
  if (!result) return {};
  return {
    ...result,
    songs: (result.songs ?? []).map((t) => ({ ...t, sourceId })),
    playlists: (result.playlists ?? []).map((p) => ({ ...p, sourceId })),
  };
}

function normalizeRecommendResult(result: RecommendResult | null, sourceId: string): RecommendResult {
  if (!result) return {};
  return {
    ...result,
    songs: (result.songs ?? []).map((t) => ({ ...t, sourceId })),
    playlists: (result.playlists ?? []).map((p) => ({ ...p, sourceId })),
  };
}

/** 供外部（如视图）判断插件来源显示名 */
export function pluginDisplayName(records: PluginRecord[], sourceId?: string): string {
  if (!sourceId) return "";
  return records.find((r) => r.entry.id === sourceId)?.entry.manifest.name ?? sourceId;
}

export type { UnifiedPlaylist };
