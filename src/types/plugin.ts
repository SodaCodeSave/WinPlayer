/**
 * WinPlayer 插件协议类型定义
 *
 * 插件 = manifest.json + JavaScript 模块（或单文件 JSON：manifest 字段 + code）。
 * 插件运行于 Web Worker 沙箱中，网络请求经 Rust 后端代理，
 * 因此插件可以把任意来源的 API 适配为本协议定义的统一格式。
 */

/** 当前插件协议版本 */
export const PLUGIN_API_VERSION = 1;

/** 插件可声明的能力（manifest.capabilities） */
export type PluginCapability =
  | "search"
  | "songUrl"
  | "playlist"
  | "recommend"
  | "lyrics"
  | "hotKeywords";

/** 插件设置项（由插件 manifest 声明，插件管理页据此渲染表单） */
export interface PluginSettingItem {
  key: string;
  label: string;
  type: "text" | "password" | "number" | "switch" | "select";
  default?: string | number | boolean;
  placeholder?: string;
  description?: string;
  options?: Array<{ label: string; value: string }>;
}

/** 插件清单 */
export interface PluginManifest {
  /** 唯一 ID，仅允许 [a-z0-9._-]，如 com.example.mysource */
  id: string;
  /** 显示名称 */
  name: string;
  /** 插件版本 */
  version: string;
  description?: string;
  author?: string;
  homepage?: string;
  /** 入口 JS 相对路径（目录型插件，缺省 main.js） */
  main?: string;
  /** 插件协议版本 */
  apiVersion: number;
  /** 声明的能力 */
  capabilities: PluginCapability[];
  /** 设置项定义 */
  settings?: PluginSettingItem[];
}

/** 单文件 JSON 插件：清单字段 + code */
export interface SingleFilePlugin extends PluginManifest {
  code: string;
}

/** 已安装插件条目（来自 Rust plugin_list 命令） */
export interface PluginEntry {
  id: string;
  manifest: PluginManifest;
  kind: "dir" | "file";
}

/* ------------------------------ 统一数据格式 ------------------------------ */

/** 统一歌曲结构 */
export interface UnifiedTrack {
  /** 来源内的歌曲 ID */
  id: string;
  name: string;
  artists: string[];
  album?: string;
  cover?: string;
  /** 时长（毫秒） */
  duration?: number;
  /** 来源插件 ID（主程序在返回给上层前统一补全） */
  sourceId?: string;
  /** 透传数据，仅插件自身可见 */
  extra?: Record<string, unknown>;
}

/** 统一歌单结构 */
export interface UnifiedPlaylist {
  id: string;
  name: string;
  cover?: string;
  description?: string;
  trackCount?: number;
  creator?: string;
  sourceId?: string;
}

/** 搜索结果 */
export interface SearchResult {
  songs?: UnifiedTrack[];
  playlists?: UnifiedPlaylist[];
  total?: number;
  hasMore?: boolean;
}

/** 播放直链 */
export interface SongUrlResult {
  url: string;
  quality?: string;
  /** 码率（bps），可选 */
  br?: number;
}

/** 歌单详情（含曲目） */
export interface PlaylistDetail extends UnifiedPlaylist {
  tracks: UnifiedTrack[];
}

/** 歌词（LRC 文本） */
export interface LyricsResult {
  lrc: string;
  /** 翻译歌词（LRC 文本，可选） */
  translation?: string;
}

/** 推荐内容 */
export interface RecommendResult {
  songs?: UnifiedTrack[];
  playlists?: UnifiedPlaylist[];
}

/* ------------------------------ 运行时接口 ------------------------------ */

/** 经主程序代理的网络请求 */
export interface PluginRequest {
  url: string;
  method?: string;
  headers?: Record<string, string>;
  body?: string;
  /** 默认自动：尝试 JSON 解析，失败回退纯文本 */
  responseType?: "auto" | "json" | "text" | "base64";
  /** 超时（毫秒），默认 15000 */
  timeout?: number;
}

/** 代理请求响应 */
export interface PluginResponse {
  status: number;
  ok: boolean;
  headers: Record<string, string>;
  /** responseType=json 时为解析后的对象 */
  data?: unknown;
  text?: string;
  /** responseType=base64 时的响应体 */
  base64?: string;
}

/** 插件初始化上下文 */
export interface PluginContext {
  /** 插件设置（manifest.settings 定义的表单值） */
  settings: Record<string, unknown>;
  /** 发起经主程序代理的网络请求（无 CORS 限制） */
  request(input: PluginRequest): Promise<PluginResponse>;
  /** 插件私有持久化存储（键值对，落盘于应用数据目录） */
  storage: {
    get<T = unknown>(key: string): Promise<T | null>;
    set(key: string, value: unknown): Promise<void>;
  };
  /** 向主程序日志面板输出（实际打印在 devtools 控制台） */
  log(...args: unknown[]): void;
}

/** 插件搜索参数 */
export interface PluginSearchParams {
  keyword: string;
  type?: "song" | "playlist" | "album";
  page?: number;
  pageSize?: number;
}

/** 插件模块统一接口：默认导出实现本接口的对象即可 */
export interface PluginModule {
  /** 初始化（可选）。可在此读取 ctx.settings、发起请求预热等 */
  init?(ctx: PluginContext): Promise<void> | void;

  /** 搜索（能力：search） */
  search?(params: PluginSearchParams): Promise<SearchResult>;

  /** 获取播放直链（能力：songUrl）。参数 id 为 UnifiedTrack.id */
  getSongUrl?(params: { id: string; quality?: string }): Promise<SongUrlResult | null>;

  /** 获取歌单详情（能力：playlist） */
  getPlaylist?(params: { id: string; page?: number }): Promise<PlaylistDetail | null>;

  /** 获取推荐内容（能力：recommend） */
  getRecommendations?(params?: { page?: number }): Promise<RecommendResult>;

  /** 获取歌词（能力：lyrics） */
  getLyrics?(params: { id: string }): Promise<LyricsResult | null>;

  /** 热搜词（能力：hotKeywords） */
  getHotKeywords?(): Promise<string[]>;

  /** 插件卸载/重载前清理（可选） */
  dispose?(): Promise<void> | void;
}

/* ------------------------------ 插件市场 ------------------------------ */

/** 市场源（设置项里维护的索引地址列表） */
export interface MarketSource {
  name: string;
  url: string;
}

/** 市场索引中的插件条目 */
export interface MarketEntry {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  /** 插件包地址（.zip 或单文件 .json） */
  url: string;
  homepage?: string;
}

/** 市场索引文件格式 */
export interface MarketIndex {
  name?: string;
  plugins: MarketEntry[];
}
