<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import QRCode from "qrcode";
import { usePluginsStore, type PluginRecord } from "@/stores/plugins";
import { useSettingsStore } from "@/stores/settings";
import { toast } from "@/services/toast";
import EmptyState from "@/components/common/EmptyState.vue";
import { ComboBox, ContentDialog, NumberBox, PasswordBox, TextBox, ToggleSwitch } from "winuionweb";
import type {
  LoginPollResult,
  LoginProfile,
  LoginState,
  MarketEntry,
  PluginSettingItem,
} from "@/types/plugin";

const plugins = usePluginsStore();
const settings = useSettingsStore();

const tab = ref<"installed" | "market">("installed");
const urlValue = ref("");
const marketLoading = ref(false);
const marketEntries = ref<MarketEntry[]>([]);
const uninstallTarget = ref<PluginRecord | null>(null);

/** ContentDialog 通过 ShowAsync() 打开（库没有 IsOpen prop） */
interface DialogRef {
  ShowAsync: () => Promise<string>;
}
const urlDialogRef = ref<DialogRef | null>(null);
const uninstallDialogRef = ref<DialogRef | null>(null);

const installedCount = computed(() => plugins.records.length);

/** 插件设置的编辑草稿（避免每敲一个字就重建插件运行时） */
const drafts = ref<Record<string, unknown>>({});
const draftTimers = new Map<string, number>();
const DRAFT_DEBOUNCE_MS = 600;

function capabilityLabel(capability: string): string {
  const map: Record<string, string> = {
    search: "搜索",
    songUrl: "播放直链",
    playlist: "歌单",
    album: "专辑",
    artist: "歌手",
    recommend: "推荐",
    lyrics: "歌词",
    hotKeywords: "热搜词",
    login: "账号登录",
    avatar: "头像",
    myPlaylists: "个人歌单",
  };
  return map[capability] ?? capability;
}

function draftKey(record: PluginRecord, item: PluginSettingItem): string {
  return `${record.entry.id}:${item.key}`;
}

function storedValue(record: PluginRecord, item: PluginSettingItem): string | number | boolean {
  const value = record.settings[item.key];
  if (item.type === "switch") return Boolean(value);
  if (item.type === "number") return Number(value ?? item.default ?? 0);
  return value === undefined || value === null ? "" : String(value);
}

function draftValue(record: PluginRecord, item: PluginSettingItem): string | number | boolean {
  const key = draftKey(record, item);
  return key in drafts.value
    ? (drafts.value[key] as string | number | boolean)
    : storedValue(record, item);
}

/** 更新草稿并防抖持久化（持久化会重建插件 Worker，不能每键触发） */
function setDraft(record: PluginRecord, item: PluginSettingItem, value: unknown): void {
  const key = draftKey(record, item);
  drafts.value[key] = value;
  const timer = draftTimers.get(key);
  if (timer) window.clearTimeout(timer);
  draftTimers.set(
    key,
    window.setTimeout(() => {
      draftTimers.delete(key);
      if (record.settings[item.key] === value) return;
      void plugins.updateSettings(record.entry.id, { ...record.settings, [item.key]: value });
    }, DRAFT_DEBOUNCE_MS),
  );
}

/** 开关类设置立即生效 */
function setSwitch(record: PluginRecord, item: PluginSettingItem, value: boolean): void {
  const key = draftKey(record, item);
  drafts.value[key] = value;
  void plugins.updateSettings(record.entry.id, { ...record.settings, [item.key]: value });
}

/* select 类型：ComboBox 用标签展示，写回值 */
function selectLabels(item: PluginSettingItem): string[] {
  return (item.options ?? []).map((option) => option.label);
}

function selectIndex(record: PluginRecord, item: PluginSettingItem): number {
  const current = String(draftValue(record, item));
  return (item.options ?? []).findIndex((option) => option.value === current);
}

function onSelectChange(
  record: PluginRecord,
  item: PluginSettingItem,
  ...args: unknown[]
): void {
  const payload = args[1];
  const added = (payload as { AddedItems?: unknown[] } | undefined)?.AddedItems;
  const label = String(added?.[0] ?? "");
  const option = (item.options ?? []).find((entry) => entry.label === label);
  if (option) setDraft(record, item, option.value);
}

/* ------------------------------ 扫码登录（能力：login） ------------------------------ */

/** 正在展示的二维码会话 */
interface QrSession {
  image: string;
  key?: string;
}

const loginStates = ref<Record<string, LoginState | null>>({});
const loginQr = ref<Record<string, QrSession | null>>({});
const loginStatus = ref<Record<string, string>>({});
const loginErrors = ref<Record<string, string>>({});
const loginBusy = ref<Record<string, boolean>>({});

/** 每个插件的轮询与过期定时器 */
const pollTimers = new Map<string, number>();
const expiryTimers = new Map<string, number>();

function supportsLogin(record: PluginRecord): boolean {
  // 停用的插件拿不到运行时，登录区整块隐藏
  return record.enabled && plugins.hasCapability(record, "login");
}

function isLoggedIn(id: string): boolean {
  return loginStates.value[id]?.loggedIn === true;
}

function loginName(id: string): string {
  const nickname = loginStates.value[id]?.profile?.nickname;
  return nickname ? `已登录 · ${nickname}` : "已登录";
}

function loginAvatar(id: string): string {
  return loginStates.value[id]?.profile?.avatar ?? "";
}

function qrImage(id: string): string {
  return loginQr.value[id]?.image ?? "";
}

function qrActive(id: string): boolean {
  return Boolean(loginQr.value[id]);
}

function statusText(id: string): string {
  return loginStatus.value[id] ?? "";
}

function errorText(id: string): string {
  return loginErrors.value[id] ?? "";
}

function loginBusyNow(id: string): boolean {
  return Boolean(loginBusy.value[id]);
}

function stopPolling(id: string): void {
  const timer = pollTimers.get(id);
  if (timer) window.clearTimeout(timer);
  pollTimers.delete(id);
}

function clearQr(id: string, status = ""): void {
  stopPolling(id);
  const expiry = expiryTimers.get(id);
  if (expiry) window.clearTimeout(expiry);
  expiryTimers.delete(id);
  loginQr.value[id] = null;
  loginStatus.value[id] = status;
}

/** 查询登录态（凭证在扫码成功后由 store 写回设置，会重建 Worker，因此这里每次重新拉取） */
async function refreshLoginState(record: PluginRecord, fallback?: LoginProfile): Promise<void> {
  const id = record.entry.id;
  try {
    const state = await plugins.getLoginState(id);
    loginStates.value[id] = state ?? (fallback ? { loggedIn: true, profile: fallback } : null);
  } catch {
    loginStates.value[id] = fallback ? { loggedIn: true, profile: fallback } : null;
  }
}

async function startQrLogin(record: PluginRecord): Promise<void> {
  const id = record.entry.id;
  if (loginBusyNow(id)) return;
  clearQr(id);
  delete loginErrors.value[id];
  loginBusy.value[id] = true;
  loginStatus.value[id] = "正在获取二维码…";

  try {
    const qr = await plugins.getLoginQr(id);
    if (!qr?.content) throw new Error("插件未返回登录二维码");

    const image = await QRCode.toDataURL(qr.content, {
      margin: 1,
      width: 220,
      color: { dark: "#000000", light: "#ffffff" },
    });

    loginQr.value[id] = { image, key: qr.key };
    loginStatus.value[id] = qr.hint ?? "等待扫码";
    startPolling(record, qr.key, qr.expiresIn ?? record.entry.manifest.login?.expiresIn ?? 120000);
  } catch (error) {
    loginQr.value[id] = null;
    loginStatus.value[id] = "";
    loginErrors.value[id] = error instanceof Error ? error.message : String(error);
  } finally {
    loginBusy.value[id] = false;
  }
}

function startPolling(record: PluginRecord, key: string | undefined, expiresIn: number): void {
  const id = record.entry.id;
  stopPolling(id);
  const interval = Math.max(500, record.entry.manifest.login?.pollInterval ?? 2000);

  const schedule = (): void => {
    if (!qrActive(id)) return;
    pollTimers.set(id, window.setTimeout(() => void tick(), interval));
  };

  const tick = async (): Promise<void> => {
    pollTimers.delete(id);
    if (!qrActive(id)) return;

    let result: LoginPollResult | null = null;
    try {
      result = await plugins.pollLogin(id, key);
    } catch (error) {
      // 单次轮询失败（网络抖动）不直接判死，下一轮会继续；Worker 被重建时会自然停止
      loginErrors.value[id] = error instanceof Error ? error.message : String(error);
    }

    if (!result) {
      schedule();
      return;
    }

    switch (result.status) {
      case "waiting":
        loginStatus.value[id] = "等待扫码";
        delete loginErrors.value[id];
        break;
      case "confirming":
        loginStatus.value[id] = "已扫码，请在手机上确认";
        delete loginErrors.value[id];
        break;
      case "expired":
        clearQr(id, "二维码已过期，请重新获取");
        return;
      case "success": {
        clearQr(id);
        toast(`「${record.entry.manifest.name}」登录成功`, "success");
        await refreshLoginState(record, result.profile);
        return;
      }
      default:
        clearQr(id, result.message ?? "登录失败");
        return;
    }
    schedule();
  };

  schedule();
  // 服务端返回 800 之前就到期时的兜底
  expiryTimers.set(
    id,
    window.setTimeout(() => clearQr(id, "二维码已过期，请重新获取"), expiresIn),
  );
}

function cancelQr(record: PluginRecord): void {
  clearQr(record.entry.id);
}

async function confirmLogout(record: PluginRecord): Promise<void> {
  const id = record.entry.id;
  clearQr(id);
  try {
    await plugins.logout(id);
    loginStates.value[id] = { loggedIn: false };
    loginStatus.value[id] = "已退出登录";
    toast(`「${record.entry.manifest.name}」已退出登录`, "success");
  } catch (error) {
    loginErrors.value[id] = error instanceof Error ? error.message : String(error);
  }
}

async function refreshAllLoginStates(): Promise<void> {
  await Promise.allSettled(
    plugins.records.filter(supportsLogin).map((record) => refreshLoginState(record)),
  );
}

async function installFromFile() {
  await plugins.installFromDialog();
}

async function openUrlDialog() {
  urlValue.value = "";
  const result = await urlDialogRef.value?.ShowAsync();
  if (result !== "Primary") return;
  const url = urlValue.value.trim();
  if (!url) {
    toast("请输入插件地址", "error");
    return;
  }
  await plugins.installFromUrl(url);
}

async function confirmUninstall(record: PluginRecord) {
  uninstallTarget.value = record;
  const result = await uninstallDialogRef.value?.ShowAsync();
  uninstallTarget.value = null;
  if (result === "Primary") await plugins.uninstall(record.entry.id);
}

function onToggle(record: PluginRecord, event: { IsOn: boolean }): void {
  void plugins.togglePlugin(record.entry.id, Boolean(event?.IsOn));
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

// 插件列表可能异步加载/安装，按“具备登录能力的插件 id 集合”变化时补查一次登录态
watch(
  () =>
    plugins.records
      .filter(supportsLogin)
      .map((record) => record.entry.id)
      .join(","),
  () => void refreshAllLoginStates(),
  { immediate: true },
);

onMounted(async () => {
  if (!plugins.records.length && !plugins.loading) await plugins.loadInstalled();
  await loadMarket();
  await refreshAllLoginStates();
});

onUnmounted(() => {
  for (const timer of draftTimers.values()) window.clearTimeout(timer);
  draftTimers.clear();
  for (const timer of pollTimers.values()) window.clearTimeout(timer);
  pollTimers.clear();
  for (const timer of expiryTimers.values()) window.clearTimeout(timer);
  expiryTimers.clear();
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
        <button class="wp-standard-button small" @click="openUrlDialog">
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
            <ToggleSwitch :IsOn="record.enabled" @Toggled="onToggle(record, $event)" />
            <button class="wp-standard-button small danger" @click="confirmUninstall(record)">
              卸载
            </button>
          </div>

          <!-- 扫码登录（能力：login） -->
          <div v-if="supportsLogin(record)" class="plugin-login">
            <template v-if="isLoggedIn(record.entry.id)">
              <div class="login-user">
                <img
                  v-if="loginAvatar(record.entry.id)"
                  class="login-avatar"
                  :src="loginAvatar(record.entry.id)"
                  alt=""
                />
                <span v-else class="wp-icon login-icon">&#xE77B;</span>
                <span class="login-name">{{ loginName(record.entry.id) }}</span>
                <span v-if="statusText(record.entry.id)" class="login-sub">
                  {{ statusText(record.entry.id) }}
                </span>
              </div>
              <button class="wp-standard-button small" @click="confirmLogout(record)">退出登录</button>
            </template>

            <template v-else-if="qrActive(record.entry.id)">
              <div class="qr-box">
                <img class="qr-image" :src="qrImage(record.entry.id)" alt="登录二维码" />
              </div>
              <div class="qr-side">
                <div class="login-name">{{ statusText(record.entry.id) || "等待扫码" }}</div>
                <div v-if="errorText(record.entry.id)" class="login-error">
                  {{ errorText(record.entry.id) }}
                </div>
                <button class="wp-standard-button small" @click="cancelQr(record)">取消</button>
              </div>
            </template>

            <template v-else>
              <div class="login-idle">
                <div class="login-name">未登录</div>
                <div class="login-desc">扫码登录后可播放 VIP / 受限歌曲</div>
                <div v-if="statusText(record.entry.id)" class="login-sub">
                  {{ statusText(record.entry.id) }}
                </div>
                <div v-if="errorText(record.entry.id)" class="login-error">
                  {{ errorText(record.entry.id) }}
                </div>
              </div>
              <button
                class="wp-accent-button small"
                :disabled="loginBusyNow(record.entry.id)"
                @click="startQrLogin(record)"
              >
                {{ loginBusyNow(record.entry.id) ? "获取中…" : "扫码登录" }}
              </button>
            </template>
          </div>

          <!-- 插件设置表单 -->
          <div v-if="record.entry.manifest.settings?.length" class="plugin-settings">
            <div v-for="item in record.entry.manifest.settings" :key="item.key" class="setting-row">
              <div class="setting-label">
                <div class="setting-name">{{ item.label }}</div>
                <div v-if="item.description" class="setting-desc">{{ item.description }}</div>
              </div>
              <TextBox
                v-if="item.type === 'text'"
                class="setting-control"
                :Text="String(draftValue(record, item))"
                :PlaceholderText="item.placeholder ?? ''"
                @update:Text="setDraft(record, item, $event)"
              />
              <PasswordBox
                v-else-if="item.type === 'password'"
                class="setting-control"
                :Password="String(draftValue(record, item))"
                :PlaceholderText="item.placeholder ?? ''"
                @update:Password="setDraft(record, item, $event)"
              />
              <NumberBox
                v-else-if="item.type === 'number'"
                class="setting-control"
                :Value="Number(draftValue(record, item)) || 0"
                @update:Value="setDraft(record, item, $event)"
              />
              <ComboBox
                v-else-if="item.type === 'select'"
                class="setting-control"
                :ItemsSource="selectLabels(item)"
                :SelectedIndex="selectIndex(record, item)"
                @SelectionChanged="
                  (sender: unknown, payload: unknown) => onSelectChange(record, item, sender, payload)
                "
              />
              <ToggleSwitch
                v-else-if="item.type === 'switch'"
                :IsOn="Boolean(draftValue(record, item))"
                @Toggled="setSwitch(record, item, Boolean($event?.IsOn))"
              />
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
      ref="urlDialogRef"
      Title="从 URL 安装插件"
      PrimaryButtonText="安装"
      CloseButtonText="取消"
    >
      <div class="dialog-form">
        <TextBox
          :Text="urlValue"
          PlaceholderText="https://example.com/my-plugin.zip"
          @update:Text="urlValue = String($event ?? '')"
        />
        <div class="dialog-hint">支持 .zip 插件包或单文件 .json 插件的直链地址</div>
      </div>
    </ContentDialog>

    <ContentDialog
      ref="uninstallDialogRef"
      :Title="`卸载插件「${uninstallTarget?.entry.manifest.name ?? ''}」`"
      PrimaryButtonText="卸载"
      CloseButtonText="取消"
    >
      <div class="dialog-hint">卸载后将删除插件本体与其存储的数据，此操作不可撤销。</div>
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

/**
 * 卡片使用两列网格：主信息占第一列，开关/操作占第二列，
 * 设置表单独占整行（此前用 flex + flex-basis:100% 会被挤进同一行导致排版错乱）
 */
.plugin-card {
  display: grid;
  grid-template-columns: 1fr auto;
  align-content: start;
  gap: var(--space-lg) var(--space-xl);
  padding: var(--space-xl);
  background: var(--wp-card);
  border: 1px solid var(--wp-control-stroke);
  border-radius: var(--radius-lg);
}

.plugin-main {
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
}

/* 扫码登录区：与设置表单一样占满整行 */
.plugin-login {
  grid-column: 1 / -1;
  border-top: 1px solid var(--wp-divider);
  padding-top: var(--space-lg);
  display: flex;
  align-items: center;
  gap: var(--space-xl);
  min-height: 56px;
}

.login-user {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  min-width: 0;
  flex: 1;
}

.login-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  object-fit: cover;
  flex: none;
}

.login-icon {
  font-size: 22px;
  color: var(--wp-text-secondary);
  flex: none;
}

.login-idle {
  flex: 1;
  min-width: 0;
}

.qr-box {
  flex: none;
  padding: 6px;
  background: #ffffff;
  border: 1px solid var(--wp-control-stroke-secondary);
  border-radius: var(--radius-sm);
  line-height: 0;
}

.qr-image {
  width: 132px;
  height: 132px;
  image-rendering: pixelated;
}

.qr-side {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--space-md);
}

.login-name {
  font-size: 13px;
  font-weight: 600;
}

.login-desc {
  font-size: 12px;
  color: var(--wp-text-secondary);
  margin-top: 2px;
}

.login-sub {
  font-size: 12px;
  color: var(--wp-text-tertiary);
}

.login-error {
  font-size: 12px;
  color: var(--wp-danger);
  margin-top: 2px;
}

.plugin-settings {
  grid-column: 1 / -1;
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

.setting-control {
  width: 280px;
  flex: none;
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

.dialog-hint {
  font-size: 12px;
  color: var(--wp-text-tertiary);
}
</style>
