<script setup lang="ts">
import { computed } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useSettingsStore } from "@/stores/settings";
import { useUpdaterStore } from "@/stores/updater";

const settings = useSettingsStore();
const updater = useUpdaterStore();

/** 当前版本号：启动时由 updater.init() 读取；未就绪时回退占位 */
const versionText = computed(() => updater.currentVersion || "—");

const busy = computed(
  () =>
    updater.phase === "checking" ||
    updater.phase === "downloading" ||
    updater.phase === "installing",
);

/** 「检查更新」按钮文案 */
const checkButtonText = computed(() => {
  if (updater.phase === "checking") return "检查中…";
  if (updater.phase === "downloading") return "正在下载…";
  if (updater.phase === "installing") return "正在安装…";
  if (updater.hasUpdate) return "查看更新";
  return "检查更新";
});

/** 更新状态说明 */
const updateStatusText = computed(() => {
  if (updater.phase === "checking") return "正在检查更新…";
  if (updater.phase === "downloading") return "正在下载最新版本…";
  if (updater.phase === "installing") return "正在安装更新，应用即将重启…";
  if (updater.phase === "error") return updater.error ?? "检查更新失败";

  if (updater.hasUpdate && updater.info) {
    const base = `发现新版本 ${updater.info.version}`;
    return updater.ignoredVersion
      ? `${base}（已忽略自动提示，可手动更新）`
      : base;
  }
  if (updater.unsupportedVersion) {
    const target = updater.platform
      ? `${updater.platform.platform}-${updater.platform.arch}`
      : "当前平台";
    return `发现新版本 ${updater.unsupportedVersion}，但该版本没有 ${target} 的安装包`;
  }
  if (updater.phase === "done") return "更新已完成";

  if (!updater.canAutoUpdate) {
    return "当前平台不支持自动更新，可前往 Release 页面手动下载";
  }
  if (settings.lastUpdateCheckAt) {
    const at = new Date(settings.lastUpdateCheckAt).toLocaleString();
    return `当前已是最新版本 · 上次检查：${at}`;
  }
  return "尚未检查更新";
});

/** 检查更新：已发现新版本时打开确认弹窗，否则重新查询 */
function onCheckUpdate(): void {
  if (updater.hasUpdate) updater.openPrompt();
  else void updater.check();
}

function openReleases(): void {
  const url = updater.platform?.releasesUrl ?? "https://github.com/WinPlayer-dev/WinPlayer/releases";
  void openUrl(url);
}

function openRepo() {
  void openUrl("https://github.com/Furry-Xiyi/WinUIonWeb");
}

function restoreIgnored(): void {
  void updater.forgetIgnoredVersion();
}
</script>

<template>
  <div>
    <!-- 关于 -->
    <section class="settings-section">
      <h2 class="settings-section-title">关于</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">WinPlayer</div>
          <div class="card-description">
            版本 {{ versionText }} · 基于插件的桌面音乐播放器 · MIT License
          </div>
        </div>
      </div>

      <!-- 自动更新 -->
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">软件更新</div>
          <div class="card-description">{{ updateStatusText }}</div>
          <div v-if="settings.ignoredUpdateVersion" class="ignored-row">
            <span>已忽略版本 {{ settings.ignoredUpdateVersion }} 的自动提示</span>
            <button class="wp-standard-button small" @click="restoreIgnored">恢复提示</button>
          </div>
        </div>
        <div class="update-actions">
          <button class="wp-standard-button small" :disabled="busy" @click="onCheckUpdate">
            {{ checkButtonText }}
          </button>
          <button class="wp-standard-button small" @click="openReleases">Release 页面</button>
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
            <p>其他依赖：Tauri（MIT/Apache-2.0）、Vue（MIT）。</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.update-actions {
  display: flex;
  gap: var(--space-md);
  flex: none;
}

.ignored-row {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-top: var(--space-md);
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.attribution p {
  margin: 0 0 var(--space-md) 0;
}

.attribution p:last-child {
  margin-bottom: 0;
}
</style>
