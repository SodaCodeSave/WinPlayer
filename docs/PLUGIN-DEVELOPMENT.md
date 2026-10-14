# WinPlayer 插件开发指南

WinPlayer 的音乐完全来自插件。插件负责把任意音乐来源（HTTP API、爬虫、私有协议等）适配为 WinPlayer 的**统一插件协议**，主程序只与统一协议打交道。

```
┌────────────┐   统一协议    ┌──────────────┐   RPC    ┌───────────────┐
│  主程序 UI  │ ◄──────────► │  插件沙箱      │ ◄──────► │ Rust 后端代理  │
│ (Vue/Pinia)│              │ (Web Worker)  │          │ (HTTP/存储)   │
└────────────┘              └──────┬───────┘          └───────┬───────┘
                                   │ import(插件代码)          │ 转发请求
                                   ▼                          ▼
                            ┌──────────────┐           任意来源 API
                            │  插件 JS 代码 │
                            └──────────────┘
```

## 安全模型

- 插件代码运行在 **Web Worker 沙箱**中：无 DOM、无 `window`、无法访问主程序状态
- 插件的**所有网络请求**必须通过 `ctx.request()`，由 Rust 后端代理转发（无 CORS 限制，也无法绕过）
- 插件只能访问自己的**私有存储** `ctx.storage`
- 可调用的方法有白名单限制（`search` / `getSongUrl` / `getPlaylist` / `getRecommendations` / `getLyrics` / `getHotKeywords`）

## 插件包格式

### 目录型插件（推荐开发时使用）

```
my-plugin/
├── manifest.json
└── main.js          # 入口文件，路径由 manifest.main 指定
```

分发时压缩为 `.zip`（`manifest.json` 位于 zip 根目录，或位于唯一的顶层文件夹内）。

### 单文件 JSON 插件

`manifest.json` 的全部字段 + `code` 字段（入口 JS 源码字符串），扩展名 `.json`。适合市场/URL 分发。

> 可运行 `node scripts/build-demo-plugin.mjs` 从目录版示例生成单文件版本。

### manifest.json 字段

```jsonc
{
  "id": "com.example.mysource",   // 必填，唯一 ID，仅允许 [a-z0-9._-]，≤64 字符
  "name": "我的音源",              // 必填，显示名称
  "version": "1.0.0",             // 必填，版本号
  "description": "简介",           // 可选
  "author": "作者",                // 可选
  "homepage": "https://...",      // 可选
  "main": "main.js",              // 可选，入口文件（默认 main.js，仅限插件目录内相对路径）
  "apiVersion": 1,                // 必填，插件协议版本（当前为 1）
  "capabilities": ["search", "songUrl", "recommend", "lyrics", "hotKeywords", "playlist"],
  "settings": [                   // 可选，声明设置项，主程序自动生成配置表单
    { "key": "token",  "label": "API Token", "type": "password", "description": "用于鉴权" },
    { "key": "area",   "label": "音质", "type": "select", "default": "standard",
      "options": [ { "label": "标准", "value": "standard" }, { "label": "高品", "value": "high" } ] },
    { "key": "debug",  "label": "调试日志", "type": "switch", "default": false }
  ]
}
```

`settings` 支持的 `type`：`text` / `password` / `number` / `switch` / `select`。
用户填写的值会以 `ctx.settings` 传入插件。

## 插件模块接口

入口文件**默认导出**一个实现以下接口的对象（ESM `export default {...}`，兼容 `module.exports = {...}`）：

```js
export default {
  /** 初始化（可选）。此处可读取设置、预热请求 */
  async init(ctx) {},

  /** 搜索（能力：search） */
  async search({ keyword, type = "song", page = 1, pageSize = 20 }) {
    return {
      songs: [/* UnifiedTrack */],
      playlists: [/* UnifiedPlaylist */],
      total: 100,        // 可选
      hasMore: true,     // 可选
    };
  },

  /** 获取播放直链（能力：songUrl）。参数 id 即 UnifiedTrack.id */
  async getSongUrl({ id, quality }) {
    return { url: "https://.../song.mp3", quality: "standard" };
    // 无法获取时返回 null
  },

  /** 歌单详情（能力：playlist） */
  async getPlaylist({ id, page }) {
    return { id, name, cover, description, trackCount, tracks: [/* UnifiedTrack */] };
  },

  /** 推荐内容（能力：recommend）——显示在“推荐”页 */
  async getRecommendations({ page } = {}) {
    return { songs: [], playlists: [] };
  },

  /** 歌词（能力：lyrics）——LRC 文本 */
  async getLyrics({ id }) {
    return { lrc: "[00:00.00] 第一行\n[00:05.00] 第二行", translation: "可选的翻译 LRC" };
  },

  /** 热搜词（能力：hotKeywords） */
  async getHotKeywords() {
    return ["关键词1", "关键词2"];
  },

  /** 卸载/重载前清理（可选） */
  async dispose() {},
};
```

### 统一数据格式

```ts
// UnifiedTrack —— 所有来源的歌曲都转换为该结构
{
  id: string;          // 来源内唯一 ID（必填）
  name: string;        // 歌名（必填）
  artists: string[];   // 艺术家列表（必填）
  album?: string;      // 专辑名
  cover?: string;      // 封面图 URL
  duration?: number;   // 时长（毫秒）
  extra?: object;      // 透传数据，仅你自己可见（如原始 ID、版权信息）
}

// UnifiedPlaylist
{
  id: string;
  name: string;
  cover?: string;
  description?: string;
  trackCount?: number;
  creator?: string;
}
```

> `sourceId` 由主程序自动补全，插件无需（也不应）设置。

### 上下文对象 ctx

```js
async init(ctx) {
  // 1. 用户在插件管理页填写的配置
  ctx.settings.token; // "abc123"

  // 2. 网络请求（经 Rust 后端代理，无 CORS；禁止使用 fetch/XMLHttpRequest）
  const res = await ctx.request({
    url: "https://api.example.com/search",
    method: "GET",                 // 默认 GET
    headers: { Authorization: `Bearer ${ctx.settings.token}` },
    body: undefined,               // 请求体（字符串）
    responseType: "auto",          // "auto"(默认,尝试 JSON) | "text" | "json" | "base64"
    timeout: 15000,                // 毫秒，默认 15000，最大 120000
  });
  res.status;   // HTTP 状态码
  res.ok;       // 2xx?
  res.headers;  // 响应头
  res.data;     // responseType 为 auto/json 时的解析结果
  res.text;     // 文本响应体
  res.base64;   // responseType=base64 时的响应体

  // 3. 私有持久化存储（键值对，落盘于应用数据目录，仅本插件可读写）
  const cached = await ctx.storage.get("cachedToken");
  await ctx.storage.set("lastSearch", { keyword: "xxx", at: Date.now() });

  // 4. 日志（输出到应用控制台）
  ctx.log("初始化完成");
}
```

## 最小可用插件

```js
// manifest.json: { "id": "demo.hello", "name": "Hello", "version": "1.0.0",
//                  "apiVersion": 1, "capabilities": ["search", "songUrl"] }
export default {
  async search({ keyword }) {
    return {
      songs: [{ id: "1", name: `来自 ${keyword} 的歌`, artists: ["歌手"], duration: 180000 }],
    };
  },
  async getSongUrl({ id }) {
    return { url: "https://example.com/audio.mp3" };
  },
};
```

打包为 zip 或单文件 JSON 后，在「插件管理 → 从本地安装 / 从 URL 安装」中安装即可。

## 插件市场索引格式

市场源是一个 JSON 文件的 URL（可在设置页添加多个）：

```json
{
  "name": "我的市场",
  "plugins": [
    {
      "id": "com.example.mysource",
      "name": "我的音源",
      "version": "1.2.0",
      "description": "简介",
      "author": "作者",
      "url": "https://example.com/my-plugin.zip",
      "homepage": "https://example.com"
    }
  ]
}
```

示例见 [market-index.example.json](./market-index.example.json)。

## 调试

1. `pnpm tauri dev` 启动开发模式
2. 在插件管理页安装 `plugins/demo-source`（或 `plugins/dist/demo-source-1.0.0.json`）
3. 右键窗口 → 检查（DevTools）查看 `[插件 com.winplayer.demo]` 日志
4. 修改插件后重新安装（同 ID 覆盖安装）即可生效

## 常见问题

- **插件里能用 fetch 吗？** 不能。必须用 `ctx.request`，它经后端代理，可跨域、可自定义请求头。
- **音频播放需要处理 CORS 吗？** `getSongUrl` 返回直链后由主程序的 `<audio>` 直接播放，无需 CORS 头；若来源要求 Referer/UA 校验，请在插件里换取宽松的直链。
- **如何实现登录？** 在插件设置中让用户填写 Cookie/Token（`type: "password"`），或在 `init` 里用 `ctx.request` + `ctx.storage` 完成登录流程并缓存会话。
- **多大的超时？** 网络请求默认 15 秒；方法调用（如 search）主程序侧 45 秒超时，请避免长时间阻塞。
