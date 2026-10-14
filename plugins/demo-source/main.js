/**
 * WinPlayer 示例音源插件
 *
 * 演示插件协议的全部核心能力：搜索 / 播放直链 / 推荐 / 歌词 / 热搜词。
 * 音频使用 SoundHelix 公开测试文件，仅供开发调试用。
 *
 * 插件运行于 Web Worker 沙箱：
 * - 无法访问 DOM / window / 主程序内部
 * - 网络请求必须通过 ctx.request（由主程序经 Rust 后端代理）
 * - 持久化必须通过 ctx.storage
 */

/** 内置“曲库”（模拟任意来源 API 的返回数据） */
const CATALOG = [
  { id: "sh-1", name: "SoundHelix Song 1", artists: ["Thomas Schürger"], album: "SoundHelix", duration: 372000, audio: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3" },
  { id: "sh-2", name: "SoundHelix Song 2", artists: ["Thomas Schürger"], album: "SoundHelix", duration: 424000, audio: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-2.mp3" },
  { id: "sh-3", name: "SoundHelix Song 3", artists: ["Thomas Schürger"], album: "SoundHelix", duration: 346000, audio: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-3.mp3" },
  { id: "sh-4", name: "SoundHelix Song 4", artists: ["Thomas Schürger"], album: "SoundHelix", duration: 401000, audio: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-4.mp3" },
  { id: "sh-5", name: "SoundHelix Song 5", artists: ["Thomas Schürger"], album: "SoundHelix", duration: 358000, audio: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-5.mp3" },
  { id: "sh-6", name: "SoundHelix Song 6", artists: ["Thomas Schürger"], album: "SoundHelix", duration: 432000, audio: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-6.mp3" },
  { id: "sh-7", name: "SoundHelix Song 7", artists: ["Thomas Schürger"], album: "SoundHelix", duration: 389000, audio: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-7.mp3" },
  { id: "sh-8", name: "SoundHelix Song 8", artists: ["Thomas Schürger"], album: "SoundHelix", duration: 416000, audio: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-8.mp3" },
];

/** 模拟来源 API 的数据格式：字段名与统一格式不同，由插件负责转换 */
function fromRaw(raw) {
  return {
    id: raw.id,
    name: raw.name,
    artists: raw.artists,
    album: raw.album,
    duration: raw.duration,
  };
}

function maxResults(ctx) {
  const value = Number(ctx?.settings?.maxResults);
  return Number.isFinite(value) && value > 0 ? Math.min(50, value) : 12;
}

let context = null;

export default {
  async init(ctx) {
    context = ctx;
    ctx.log(`${ctx.settings?.greeting ?? "你好"}，示例音源已就绪（目录共 ${CATALOG.length} 首）`);
  },

  /** 搜索（能力：search） */
  async search({ keyword = "", page = 1 }) {
    const kw = String(keyword).trim().toLowerCase();
    const matched = CATALOG.filter(
      (item) =>
        !kw ||
        item.name.toLowerCase().includes(kw) ||
        item.artists.some((artist) => artist.toLowerCase().includes(kw)),
    );
    const pageSize = maxResults(context);
    const start = (Math.max(1, page) - 1) * pageSize;
    const pageItems = matched.slice(start, start + pageSize);
    return {
      songs: pageItems.map(fromRaw),
      total: matched.length,
      hasMore: start + pageItems.length < matched.length,
    };
  },

  /** 获取播放直链（能力：songUrl）。真实来源可能还需请求 API 换取带签名的地址 */
  async getSongUrl({ id }) {
    const item = CATALOG.find((entry) => entry.id === id);
    if (!item) return null;
    return { url: item.audio, quality: "standard" };
  },

  /** 推荐（能力：recommend） */
  async getRecommendations() {
    const shuffled = [...CATALOG].sort(() => Math.random() - 0.5);
    return { songs: shuffled.slice(0, maxResults(context)).map(fromRaw) };
  },

  /** 歌词（能力：lyrics）：演示 LRC 格式 */
  async getLyrics({ id }) {
    if (context?.settings?.enableLyrics === false) return null;
    const item = CATALOG.find((entry) => entry.id === id);
    if (!item) return null;
    const lines = [
      "[00:00.00] " + item.name,
      "[00:04.00] （示例歌词，由插件生成）",
      "[00:09.50] 真实来源可在此返回 LRC 全文",
      "[00:15.00] 亦可通过 ctx.request 从 API 拉取",
      "[00:21.00] WinPlayer 会自动解析并滚动显示",
    ];
    return { lrc: lines.join("\n") };
  },

  /** 热搜词（能力：hotKeywords） */
  async getHotKeywords() {
    return ["Song 1", "Song 2", "SoundHelix"];
  },
};
