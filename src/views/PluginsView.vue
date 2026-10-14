<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { usePluginsStore, type PluginRecord } from "@/stores/plugins";
import { useSettingsStore } from "@/stores/settings";
import { toast } from "@/services/toast";
import EmptyState from "@/components/common/EmptyState.vue";
import { ContentDialog, ToggleSwitch } from "winuionweb";
import type { MarketEntry, PluginSettingItem } from "@/types/plugin";

const plugins = usePluginsStore();
const settings = useSettingsStore();

const tab = ref<"installed" | "market">("installed");
const urlDialogOpen = ref(false);
const urlValue = ref("");
const installingUrl = ref(false);
const marketLoading = ref(false);
const marketEntries = ref<MarketEntry[]>([]);

const installedCount = computed(() => plugins.records.length);

function capabilityLabel(capability: string): string {
  const map: Record<string, string> = {
    search: "搜索",
    songUrl: "播放直链",
    playlist: "歌单",
    recommend: "推荐",
    lyrics: "歌词",
    hotKeywords: "热搜词",
  };
  return map[capability] ?? capability;
}

async function installFromFile() {
  await plugins.installFromDialog();
}

async function confirmInstallFromUrl() {
  const url = urlValue.value.trim();
  if (!url) return;
  installingUrl.value = true;
  try {
    const ok = await plugins.installFromUrl(url);
    if (ok) {
      urlDialogOpen.value = false;
      urlValue.value = "";
    }
  } finally {
    installingUrl.value = false;
  }
}

async function confirmUninstall(record: PluginRecord) {
  if (!window.confirm(`确定要卸载插件「${record.entry.manifest.name}」吗？`)) return;
  await plugins.uninstall(record.entry.id);
}

function onToggle(record: PluginRecord, event: Event) {
  const enabled = (event.target as HTMLInputElement).checked;
  void plugins.togglePlugin(record.entry.id, enabled);
}

/** 设置表单取值 */
function settingValue(record: PluginRecord, item: PluginSettingItem): string | boolean {
  const value = record.settings[item.key];
  if (item.type === "switch") return Boolean(value);
  return value === undefined || value === null ? "" : String(value);
}

function saveSetting(record: PluginRecord, item: PluginSettingItem, rawValue: string | boolean) {
  let value: string | number | boolean = rawValue;
  if (item.type === "number") {
    const parsed = Number(rawValue);
    value = Number.isFinite(parsed) ? parsed : 0;
  } else if (item.type === "switch") {
    value = Boolean(rawValue);
  }
  void plugins.updateSettings(record.entry.id, { ...record.settings, [item.key]: value });
}

async function loadMarket() {
  if (!settings.marketSources.length) return;
  marketLoading.value = true;
  try {
    marketEntries.value = await plugins.fetchMarketEntries(settings.marketSources);
  } catch (error) {
    toast(`加载插件市场失败: ${error}`, "error");
  } finally {
    marketLoading.value = false;
  }
}

function marketInstalled(entry: MarketEntry): boolean {
  const record = plugins.records.find((r) => r.entry.id === entry.id);
  if (!record) return false;
  return record.entry.manifest.version === entry.version;
}

async function installFromMarket(entry: MarketEntry) {
  await plugins.installFromUrl(entry.url);
}

onMounted(async () => {
  if (!plugins.records.length && !plugins.loading) await plugins.loadInstalled();
  await loadMarket();
});
</script>

<template>
  <div class="wp-page">
    <div class="wp-page-header">
      <h1 class="wp-page-title">插件管理</h1>
      <span class="page-sub">已安装 {{ installedCount }} 个</span>
    </div>

    <!-- 标签页 -->
    <div class="tabs">
      <button class="tab" :class="{ active: tab === 'installed' }" @click="tab = 'installed'">已安装</button>
      <button class="tab" :class="{ active: tab === 'market' }" @click="tab = 'market'">
        插件市场
      </button>
      <div class="tab-actions">
        <button class="wp-standard-button small" @click="installFromFile">
          <span class="wp-icon">&#xE7B8;</span>从本地安装
        </button>
        <button class="wp-standard-button small" @click="urlDialogOpen = true">
          <span class="wp-icon">&#xE774;</span>从 URL 安装
        </button>
      </div>
    </div>

    <!-- 已安装列表 -->
    <template v-if="tab === 'installed'">
      <div v-if="plugins.loading" class="hint">加载中…</div>
      <EmptyState
        v-else-if="!plugins.records.length"
        icon="&#xE943;"
        title="尚未安装任何插件"
        description="插件决定音乐来自哪里。你可以从本地导入 .zip/.json 插件包、通过 URL 安装，或在插件市场中浏览。"
      >
        <button class="wp-accent-button" @click="installFromFile">从本地安装</button>
        <button class="wp-standard-button" @click="tab = 'market'">浏览市场</button>
      </EmptyState>

      <div v-else class="plugin-list">
        <div v-for="record in plugins.records" :key="record.entry.id" class="plugin-card">
          <div class="plugin-main">
            <div class="plugin-title-row">
              <span class="plugin-name">{{ record.entry.manifest.name }}</span>
              <span class="plugin-version">v{{ record.entry.manifest.version }}</span>
              <span v-if="record.entry.kind === 'file'" class="plugin-kind">单文件</span>
            </div>
            <div class="plugin-desc">{{ record.entry.manifest.description || "无描述" }}</div>
            <div class="plugin-chips">
              <span
                v-for="capability in record.entry.manifest.capabilities ?? []"
                :key="capability"
                class="chip"
              >
                {{ capabilityLabel(capability) }}
              </span>
            </div>
            <div class="plugin-meta">
              <span v-if="record.entry.manifest.author">作者：{{ record.entry.manifest.author }}</span>
              <span class="plugin-id">ID: {{ record.entry.id }}</span>
            </div>
            <div v-if="record.error" class="plugin-error">运行错误：{{ record.error }}</div>
          </div>

          <div class="plugin-side">
            <ToggleSwitch
              :IsOn="record.enabled"
              :IsEnabled="true"
              @change="onToggle(record, $event)"
            />
            <button class="wp-standard-button small danger" @click="confirmUninstall(record)">
              卸载
            </button>
          </div>

          <!-- 插件设置表单 -->
          <div v-if="record.enabled && record.entry.manifest.settings?.length" class="plugin-settings">
            <div v-for="item in record.entry.manifest.settings" :key="item.key" class="setting-row">
              <div class="setting-label">
                <div class="setting-name">{{ item.label }}</div>
                <div v-if="item.description" class="setting-desc">{{ item.description }}</div>
              </div>
              <input
                v-if="item.type === 'text' || item.type === 'password' || item.type === 'number'"
                class="setting-input"
                :type="item.type === 'password' ? 'password' : item.type === 'number' ? 'number' : 'text'"
                :value="settingValue(record, item) as string"
                :placeholder="item.placeholder"
                @change="saveSetting(record, item, ($event.target as HTMLInputElement).value)"
              />
              <select
                v-else-if="item.type === 'select'"
                class="setting-input"
                :value="settingValue(record, item) as string"
                @change="saveSetting(record, item, ($event.target as HTMLSelectElement).value)"
              >
                <option v-for="option in item.options ?? []" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
              <label v-else-if="item.type === 'switch'" class="setting-switch">
                <input
                  type="checkbox"
                  :checked="settingValue(record, item) as boolean"
                  @change="saveSetting(record, item, ($event.target as HTMLInputElement).checked)"
                />
              </label>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- 插件市场 -->
    <template v-else>
      <div v-if="!settings.marketSources.length" class="market-empty">
        <EmptyState
          icon="&#xE721;"
          title="还没有配置插件市场源"
          description="插件市场是一个 JSON 索引地址列表（由社区维护）。在设置中添加市场源地址后，即可在这里浏览和一键安装插件。"
        />
      </div>
      <template v-else>
        <div class="market-toolbar">
          <button class="wp-standard-button small" :disabled="marketLoading" @click="loadMarket">
            <span class="wp-icon">&#xE72C;</span>{{ marketLoading ? "加载中…" : "刷新市场" }}
          </button>
        </div>
        <div v-if="!marketEntries.length" class="hint">
          {{ marketLoading ? "正在加载市场索引…" : "市场索引为空或加载失败" }}
        </div>
        <div v-else class="plugin-list">
          <div v-for="entry in marketEntries" :key="entry.id + entry.url" class="plugin-card market-card">
            <div class="plugin-main">
              <div class="plugin-title-row">
                <span class="plugin-name">{{ entry.name }}</span>
                <span class="plugin-version">v{{ entry.version }}</span>
              </div>
              <div class="plugin-desc">{{ entry.description || "无描述" }}</div>
              <div class="plugin-meta">
                <span v-if="entry.author">作者：{{ entry.author }}</span>
              </div>
            </div>
            <div class="plugin-side">
              <button
                v-if="!marketInstalled(entry)"
                class="wp-accent-button small"
                @click="installFromMarket(entry)"
              >
                安装
              </button>
              <span v-else class="installed-mark">
                <span class="wp-icon">&#xE73E;</span>已安装
              </span>
              <button v-if="entry.homepage" class="wp-standard-button small" @click="openUrl(entry.homepage!)">
                主页
              </button>
            </div>
          </div>
        </div>
      </template>
    </template>

    <ContentDialog
      :IsOpen="urlDialogOpen"
      Title="从 URL 安装插件"
      PrimaryButtonText="安装"
      CloseButtonText="取消"
      @PrimaryButtonClick="confirmInstallFromUrl"
      @CloseButtonClick="urlDialogOpen = false"
    >
      <div class="dialog-form">
        <input
          v-model="urlValue"
          class="dialog-input"
          type="text"
          placeholder="https://example.com/my-plugin.zip"
        />
        <div class="dialog-hint">支持 .zip 插件包或单文件 .json 插件的直链地址</div>
        <div v-if="installingUrl" class="dialog-hint">正在下载安装…</div>
      </div>
    </ContentDialog>
  </div>
</template>

<style scoped>
.page-sub {
  font-size: 13px;
  color: var(--wp-text-secondary);
}

.tabs {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  border-bottom: 1px solid var(--wp-divider);
  margin-bottom: var(--space-xl);
}

.tab {
  border: none;
  background: transparent;
  color: var(--wp-text-secondary);
  font-size: 14px;
  padding: var(--space-md) var(--space-lg);
  cursor: pointer;
  position: relative;
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
}

.tab:hover {
  background: var(--wp-card-hover);
  color: var(--wp-text-primary);
}

.tab.active {
  color: var(--wp-text-primary);
  font-weight: 600;
}

.tab.active::after {
  content: "";
  position: absolute;
  left: var(--space-lg);
  right: var(--space-lg);
  bottom: 0;
  height: 3px;
  border-radius: 2px;
  background: var(--wp-accent);
}

.tab-actions {
  margin-left: auto;
  display: flex;
  gap: var(--space-md);
}

.small {
  height: 28px;
  font-size: 12px;
  padding: 0 12px;
}

.hint {
  padding: var(--space-3xl);
  text-align: center;
  color: var(--wp-text-tertiary);
  font-size: 13px;
}

.plugin-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.plugin-card {
  display: flex;
  gap: var(--space-xl);
  padding: var(--space-xl);
  background: var(--wp-card);
  border: 1px solid var(--wp-control-stroke);
  border-radius: var(--radius-lg);
}

.plugin-main {
  flex: 1;
  min-width: 0;
}

.plugin-title-row {
  display: flex;
  align-items: baseline;
  gap: var(--space-md);
}

.plugin-name {
  font-size: 15px;
  font-weight: 600;
}

.plugin-version {
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.plugin-kind {
  font-size: 11px;
  color: var(--wp-text-tertiary);
  border: 1px solid var(--wp-control-stroke-secondary);
  border-radius: 8px;
  padding: 1px 8px;
}

.plugin-desc {
  font-size: 13px;
  color: var(--wp-text-secondary);
  margin-top: var(--space-sm);
  line-height: 1.5;
}

.plugin-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-sm);
  margin-top: var(--space-md);
}

.chip {
  font-size: 11px;
  color: var(--wp-accent);
  background: var(--wp-accent-light);
  border-radius: 10px;
  padding: 2px 10px;
}

.plugin-meta {
  display: flex;
  gap: var(--space-lg);
  margin-top: var(--space-md);
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.plugin-error {
  margin-top: var(--space-md);
  font-size: 12px;
  color: var(--wp-danger);
}

.plugin-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--space-md);
  flex: none;
}

.plugin-settings {
  flex-basis: 100%;
  border-top: 1px solid var(--wp-divider);
  padding-top: var(--space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.setting-row {
  display: flex;
  align-items: center;
  gap: var(--space-xl);
}

.setting-label {
  flex: 1;
  min-width: 0;
}

.setting-name {
  font-size: 13px;
  font-weight: 600;
}

.setting-desc {
  font-size: 12px;
  color: var(--wp-text-secondary);
  margin-top: 2px;
}

.setting-input {
  width: 280px;
  height: 32px;
  padding: 0 var(--space-md);
  border: 1px solid var(--wp-control-stroke-secondary);
  border-radius: var(--radius-sm);
  background: var(--wp-control-fill);
  color: var(--wp-text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
}

.setting-input:focus {
  background: var(--wp-layer-solid);
  border-bottom: 2px solid var(--wp-accent);
}

.market-toolbar {
  margin-bottom: var(--space-lg);
}

.market-card {
  align-items: center;
}

.installed-mark {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  font-size: 12px;
  color: var(--wp-success);
}

.dialog-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  padding: var(--space-md) 0;
}

.dialog-input {
  height: 32px;
  padding: 0 var(--space-md);
  border: 1px solid var(--wp-control-stroke-secondary);
  border-radius: var(--radius-sm);
  background: var(--wp-control-fill);
  color: var(--wp-text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  user-select: text;
}

.dialog-hint {
  font-size: 12px;
  color: var(--wp-text-tertiary);
}
</style>
