<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ComboBox, TextBox, ToggleSwitch } from "winuionweb";
import { useSettingsStore, DEFAULT_SOURCE_SEARCH_FORMAT, type ThemePreference } from "@/stores/settings";
import { usePluginsStore, OFFICIAL_MARKET_SOURCE } from "@/stores/plugins";
import { toast } from "@/services/toast";
import { clearDataCache, dataCacheSize } from "@/services/cache";
import type { MarketSource, PluginCapability } from "@/types/plugin";
import { useIsWindows } from "@/composables/useIsMobile";

const settings = useSettingsStore();
const pluginsStore = usePluginsStore();
// SMTC 是 Windows 专属能力，其他平台隐藏该设置项
const isWindows = useIsWindows();

const newSourceName = ref("");
const newSourceUrl = ref("");

/** GitHub 镜像源编辑草稿（进入设置页时从已加载的设置初始化） */
const mirrorDraft = ref("");

/** 缓存占用（进入设置页与清理后重新统计） */
const imageCount = ref(0);
const imageBytes = ref(0);
const dataBytes = ref(0);
const cacheLoading = ref(false);

interface CacheStats {
  imageCount: number;
  imageBytes: number;
}

async function refreshCacheStats(): Promise<void> {
  cacheLoading.value = true;
  try {
    const stats = await invoke<CacheStats>("cache_stats");
    imageCount.value = stats.imageCount;
    imageBytes.value = stats.imageBytes;
    dataBytes.value = await dataCacheSize();
  } catch {
    /* 统计失败时保持原值 */
  } finally {
    cacheLoading.value = false;
  }
}

async function clearCache(): Promise<void> {
  cacheLoading.value = true;
  try {
    await invoke<CacheStats>("cache_clear");
    await clearDataCache();
    toast("缓存已清除", "success");
    await refreshCacheStats();
  } catch (error) {
    toast(`清除缓存失败: ${error}`, "error");
  } finally {
    cacheLoading.value = false;
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value < 10 && unit > 0 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

const cacheSummary = computed(
  () =>
    `封面图片 ${formatBytes(imageBytes.value)}（${imageCount.value} 张） · 页面数据 ${formatBytes(dataBytes.value)}`,
);

const THEME_OPTIONS: Array<{ label: string; value: ThemePreference }> = [
  { label: "跟随系统", value: "system" },
  { label: "浅色", value: "light" },
  { label: "深色", value: "dark" },
];

const themeLabels = computed(() => THEME_OPTIONS.map((option) => option.label));

const themeIndex = computed(() =>
  THEME_OPTIONS.findIndex((option) => option.value === settings.theme),
);

/**
 * winuionweb 的 SelectionChanged 签名为 (sender, payload)，
 * payload 中的 AddedItems 才是新选中的值。
 */
function onThemeChange(_sender: unknown, payload?: { AddedItems?: unknown[] }): void {
  const label = String(payload?.AddedItems?.[0] ?? "");
  const option = THEME_OPTIONS.find((entry) => entry.label === label);
  if (option && option.value !== settings.theme) void settings.setTheme(option.value);
}

function addSource() {
  const name = newSourceName.value.trim();
  const url = newSourceUrl.value.trim();
  if (!url) {
    toast("请输入市场源地址", "error");
    return;
  }
  const source: MarketSource = { name: name || url, url };
  void settings.setMarketSources([...settings.marketSources, source]);
  newSourceName.value = "";
  newSourceUrl.value = "";
}

function removeSource(index: number) {
  const next = settings.marketSources.filter((_, i) => i !== index);
  void settings.setMarketSources(next);
}

function openRepo() {
  void openUrl("https://github.com/Furry-Xiyi/WinUIonWeb");
}

function saveMirror() {
  void settings.setGithubMirror(mirrorDraft.value).then(() =>
    toast("GitHub 镜像源已保存", "success"),
  );
}

onMounted(() => {
  mirrorDraft.value = settings.githubMirror;
  void refreshCacheStats();
});

function onSmtcToggle(event: { IsOn: boolean }): void {
  void settings.setSmtcEnabled(Boolean(event?.IsOn));
}

/* ------------------------------ 来源偏好 ------------------------------ */

interface SourceKindConfig {
  key: "trackSourceId" | "playSourceId" | "lyricsSourceId" | "avatarSourceId";
  title: string;
  description: string;
  /** 插件需具备的能力（须全部满足才会出现在下拉框中） */
  capabilities: PluginCapability[];
}

const SOURCE_KINDS: SourceKindConfig[] = [
  {
    key: "trackSourceId",
    title: "歌曲信息源",
    description: "搜索结果聚合展示时，该插件的结果排在最前",
    capabilities: ["search"],
  },
  {
    key: "playSourceId",
    title: "播放源",
    description:
      "播放歌曲（含歌单、播放队列）时，按「搜索格式」在该插件重新搜索并播放其结果；搜索不到或解析失败时回退歌曲自身来源",
    capabilities: ["search", "songUrl"],
  },
  {
    key: "lyricsSourceId",
    title: "歌词源",
    description:
      "获取歌词时按「搜索格式」在该插件重新搜曲后取词；取不到时回退歌曲自身来源",
    capabilities: ["search", "lyrics"],
  },
  {
    key: "avatarSourceId",
    title: "头像源",
    description: "窗口右上角头像优先从该插件获取，取不到时再依次尝试其他插件",
    capabilities: ["avatar"],
  },
];

interface SourceOption {
  label: string;
  /** 插件 ID，空串表示默认（自动） */
  value: string;
}

/** 各来源下拉框选项：默认 + 按能力过滤后的已启用插件 */
const sourceOptions = computed(() =>
  SOURCE_KINDS.map((kind) => {
    const options: SourceOption[] = [{ label: "默认", value: "" }];
    for (const record of pluginsStore.enabledRecords) {
      if (kind.capabilities.every((capability) => pluginsStore.hasCapability(record, capability))) {
        options.push({ label: record.entry.manifest.name, value: record.entry.id });
      }
    }
    return { kind, options };
  }),
);

function sourceIndex(currentValue: string, options: SourceOption[]): number {
  const index = options.findIndex((option) => option.value === currentValue);
  return index >= 0 ? index : 0;
}

/**
 * 来源下拉框的行数据：每行预构造好带类型的 SelectionChanged 处理器。
 * 不能在模板里写内联箭头（模板表达式无法标注参数类型，且调用表达式会被
 * Vue 当作内联语句丢弃返回值），因此这里提前柯里化好，模板直接以标识符引用。
 */
const sourceRows = computed(() =>
  sourceOptions.value.map(({ kind, options }) => ({
    kind,
    options,
    onSelect: (sender: unknown, payload?: { AddedItems?: unknown[] }): void => {
      onSourceSelectionChanged(kind, options, sender, payload);
    },
  })),
);

function onSourceSelectionChanged(
  kind: SourceKindConfig,
  options: SourceOption[],
  _sender: unknown,
  payload?: { AddedItems?: unknown[] },
): void {
  const label = String(payload?.AddedItems?.[0] ?? "");
  const option = options.find((entry) => entry.label === label);
  if (option && option.value !== settings[kind.key]) {
    void settings.setSourcePreference(kind.key, option.value);
  }
}

/** 配置了播放源或歌词源时才显示「搜索格式」输入框（两者共用） */
const showSearchFormat = computed(() => Boolean(settings.playSourceId || settings.lyricsSourceId));

function onFormatChange(value: unknown): void {
  void settings.setSourceSearchFormat(String(value ?? ""));
}

</script>

<template>
  <div class="wp-page settings-page">
    <div class="wp-page-header">
      <h1 class="wp-page-title">设置</h1>
    </div>

    <!-- 外观 -->
    <section class="settings-section">
      <h2 class="settings-section-title">外观</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">主题</div>
          <div class="card-description">选择应用的亮/暗外观，默认跟随 Windows 系统设置</div>
        </div>
        <ComboBox
          class="card-control"
          :ItemsSource="themeLabels"
          :SelectedIndex="themeIndex"
          @SelectionChanged="onThemeChange"
        />
      </div>
    </section>

    <!-- 来源偏好 -->
    <section class="settings-section">
      <h2 class="settings-section-title">来源偏好</h2>
      <div v-for="{ kind, options, onSelect } in sourceRows" :key="kind.key" class="settings-card">
        <div class="card-content">
          <div class="card-header">{{ kind.title }}</div>
          <div class="card-description">{{ kind.description }}</div>
        </div>
        <ComboBox
          class="card-control"
          :ItemsSource="options.map((option) => option.label)"
          :SelectedIndex="sourceIndex(settings[kind.key], options)"
          @SelectionChanged="onSelect"
        />
      </div>

      <div v-if="showSearchFormat" class="settings-card column">
        <div class="card-content">
          <div class="card-header">搜索格式</div>
          <div class="card-description">
            播放源、歌词源重新搜曲时使用的关键词格式，支持变量
            {歌曲名}、{歌手名}、{专辑名}、{别名}。留空时使用默认「{{ DEFAULT_SOURCE_SEARCH_FORMAT }}」。
          </div>
        </div>
        <TextBox
          class="format-input"
          :Text="settings.sourceSearchFormat"
          :PlaceholderText="DEFAULT_SOURCE_SEARCH_FORMAT"
          @update:Text="onFormatChange"
        />
      </div>
    </section>

    <!-- 系统集成（SMTC 仅 Windows 提供） -->
    <section v-if="isWindows" class="settings-section">
      <h2 class="settings-section-title">系统集成</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">系统媒体传输控件（SMTC）</div>
          <div class="card-description">
            在 Windows 媒体浮出窗、锁屏与蓝牙设备上显示正在播放的曲目、封面与进度，并响应系统的播放 / 暂停 / 上一首 / 下一首按钮
          </div>
        </div>
        <ToggleSwitch :IsOn="settings.smtcEnabled" @Toggled="onSmtcToggle" />
      </div>
    </section>

    <!-- 存储与缓存 -->
    <section class="settings-section">
      <h2 class="settings-section-title">存储与缓存</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">清除缓存</div>
          <div class="card-description">
            当前占用：{{ cacheSummary }}。清除后封面图片和主页/歌单页数据将在下次访问时重新下载。
          </div>
        </div>
        <button class="wp-standard-button small danger" :disabled="cacheLoading" @click="clearCache">
          清除缓存
        </button>
      </div>
    </section>

    <!-- 插件市场 -->
    <section class="settings-section">
      <h2 class="settings-section-title">插件市场</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">GitHub 镜像源</div>
          <div class="card-description">
            配置后，凡是经 GitHub 下载的内容（插件仓库安装、市场索引、卡片图片）会自动在此前缀后拼接原始地址加速访问，例如 https://ghproxy.net/ 。留空则直连。
          </div>
        </div>
        <div class="mirror-edit">
          <TextBox
            class="mirror-input"
            :Text="mirrorDraft"
            PlaceholderText="https://ghproxy.net/"
            @update:Text="mirrorDraft = String($event ?? '')"
          />
          <button class="wp-standard-button small" @click="saveMirror">保存</button>
        </div>
      </div>

      <div class="settings-card column">
        <div class="card-content">
          <div class="card-header">市场源列表</div>
          <div class="card-description">
            每个市场源是一个 JSON 索引地址（格式见文档 docs/PLUGIN-DEVELOPMENT.md）。插件市场页会聚合展示所有源中的插件。
          </div>
        </div>

        <ul class="source-list">
          <li class="source-item">
            <div class="source-info">
              <div class="source-name">
                {{ OFFICIAL_MARKET_SOURCE.name }}
                <span class="builtin-badge">内置</span>
              </div>
              <div class="source-url ellipsis">{{ OFFICIAL_MARKET_SOURCE.url }}</div>
            </div>
          </li>
          <li v-for="(source, index) in settings.marketSources" :key="source.url" class="source-item">
            <div class="source-info">
              <div class="source-name">{{ source.name }}</div>
              <div class="source-url ellipsis">{{ source.url }}</div>
            </div>
            <button class="wp-standard-button small danger" @click="removeSource(index)">移除</button>
          </li>
        </ul>

        <div class="source-add">
          <TextBox
            class="source-input"
            :Text="newSourceName"
            PlaceholderText="名称（可选）"
            @update:Text="newSourceName = String($event ?? '')"
          />
          <TextBox
            class="source-input grow"
            :Text="newSourceUrl"
            PlaceholderText="https://example.com/market-index.json"
            @update:Text="newSourceUrl = String($event ?? '')"
          />
          <button class="wp-standard-button small" @click="addSource">添加</button>
        </div>
      </div>
    </section>

    <!-- 关于 -->
    <section class="settings-section">
      <h2 class="settings-section-title">关于</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">WinPlayer</div>
          <div class="card-description">版本 1.3.0 · 基于插件的桌面音乐播放器 · MIT License</div>
        </div>
      </div>

      <div class="settings-card column">
        <div class="card-content">
          <div class="card-header">开源组件声明</div>
          <div class="card-description attribution">
            <p>
              本软件的图形界面使用了开源组件库
              <a href="#" @click.prevent="openRepo">WinUIonWeb</a>
              （作者 Furry-Xiyi），该组件库以 <strong>GPL-3.0-only</strong> 协议发布，是对 Microsoft WinUI 的独立 Web 实现
            </p>
            <p>
              WinPlayer 本体源代码以 MIT 协议发布；由于分发物中包含 GPL-3.0
              组件，整个软件的分发须同时遵守 GPL-3.0 的相应义务。更多第三方协议信息见仓库中的
              THIRD_PARTY_NOTICES.md。
            </p>
            <p>
              其他依赖：Tauri（MIT/Apache-2.0）、Vue（MIT）。
            </p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
/* 滚动容器必须占满内容区（否则滚动条停在 max-width 处而非窗口右缘） */
.settings-page {
  width: 100%;
  max-width: none;
}

.settings-section {
  margin-bottom: var(--space-3xl);
}

.settings-section-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 var(--space-lg) 0;
}

.settings-card {
  display: flex;
  align-items: center;
  gap: var(--space-xl);
  padding: var(--space-xl);
  background: var(--wp-card);
  border: 1px solid var(--wp-control-stroke);
  border-radius: var(--radius-lg);
  margin-bottom: var(--space-md);
}

.settings-card.column {
  flex-direction: column;
  align-items: stretch;
}

.card-content {
  flex: 1;
  min-width: 0;
}

.card-header {
  font-size: 14px;
  font-weight: 600;
}

.card-description {
  font-size: 12px;
  color: var(--wp-text-secondary);
  margin-top: var(--space-sm);
  line-height: 1.6;
}

.attribution p {
  margin: 0 0 var(--space-md) 0;
}

.attribution p:last-child {
  margin-bottom: 0;
}

.card-control {
  width: 160px;
  flex: none;
}

.format-input {
  width: 100%;
}

.mirror-edit {
  display: flex;
  gap: var(--space-md);
  flex: none;
}

.mirror-input {
  width: 280px;
}

.source-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  width: 100%;
}

.source-item {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  padding: var(--space-md);
  background: var(--wp-control-fill-secondary);
  border-radius: var(--radius-sm);
}

.source-info {
  flex: 1;
  min-width: 0;
}

.source-name {
  font-size: 13px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.builtin-badge {
  font-size: 11px;
  font-weight: 400;
  color: var(--wp-accent);
  background: var(--wp-accent-light);
  border-radius: 8px;
  padding: 1px 8px;
}

.source-url {
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.source-add {
  display: flex;
  gap: var(--space-md);
  width: 100%;
}

.source-input {
  width: 180px;
  flex: none;
}

.source-input.grow {
  flex: 1;
  width: auto;
  min-width: 0;
}

.small {
  height: 28px;
  font-size: 12px;
  padding: 0 12px;
  flex: none;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
