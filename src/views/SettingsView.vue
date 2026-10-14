<script setup lang="ts">
import { ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useSettingsStore, type ThemePreference } from "@/stores/settings";
import { toast } from "@/services/toast";
import type { MarketSource } from "@/types/plugin";

const settings = useSettingsStore();

const newSourceName = ref("");
const newSourceUrl = ref("");

const THEME_OPTIONS: Array<{ label: string; value: ThemePreference }> = [
  { label: "跟随系统", value: "system" },
  { label: "浅色", value: "light" },
  { label: "深色", value: "dark" },
];

function onThemeChange(event: Event) {
  const value = (event.target as HTMLSelectElement).value as ThemePreference;
  void settings.setTheme(value);
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
        <select class="card-control" :value="settings.theme" @change="onThemeChange">
          <option v-for="option in THEME_OPTIONS" :key="option.value" :value="option.value">
            {{ option.label }}
          </option>
        </select>
      </div>
    </section>

    <!-- 插件市场 -->
    <section class="settings-section">
      <h2 class="settings-section-title">插件市场源</h2>
      <div class="settings-card column">
        <div class="card-content">
          <div class="card-header">市场源列表</div>
          <div class="card-description">
            每个市场源是一个 JSON 索引地址（格式见文档 docs/PLUGIN-DEVELOPMENT.md）。插件市场页会聚合展示所有源中的插件。
          </div>
        </div>

        <ul v-if="settings.marketSources.length" class="source-list">
          <li v-for="(source, index) in settings.marketSources" :key="source.url" class="source-item">
            <div class="source-info">
              <div class="source-name">{{ source.name }}</div>
              <div class="source-url ellipsis">{{ source.url }}</div>
            </div>
            <button class="wp-standard-button small danger" @click="removeSource(index)">移除</button>
          </li>
        </ul>
        <div v-else class="source-empty">还没有添加市场源</div>

        <div class="source-add">
          <input v-model="newSourceName" class="source-input" type="text" placeholder="名称（可选）" />
          <input
            v-model="newSourceUrl"
            class="source-input grow"
            type="text"
            placeholder="https://example.com/market-index.json"
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
          <div class="card-description">版本 0.1.0 · 基于插件的桌面音乐播放器 · MIT License</div>
        </div>
      </div>

      <div class="settings-card column">
        <div class="card-content">
          <div class="card-header">开源组件声明</div>
          <div class="card-description attribution">
            <p>
              本软件的图形界面使用了开源组件库
              <a href="#" @click.prevent="openRepo">WinUIonWeb</a>
              （作者 Furry-Xiyi），该组件库以 <strong>GPL-3.0-only</strong> 协议发布，是对 Microsoft
              WinUI 的独立 Web 实现，与 Microsoft 无关联，亦未获 Microsoft 认可或授权。
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
.settings-page {
  max-width: 860px;
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
}

.source-url {
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.source-empty {
  font-size: 12px;
  color: var(--wp-text-tertiary);
  padding: var(--space-md) 0;
}

.source-add {
  display: flex;
  gap: var(--space-md);
  width: 100%;
}

.source-input {
  width: 180px;
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

.source-input.grow {
  flex: 1;
  width: auto;
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
