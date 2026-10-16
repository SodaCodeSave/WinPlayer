/**
 * Windows 系统媒体传输控件（SMTC）桥接。
 *
 * 前端把播放器状态整理成快照推给 Rust（`smtc_sync`），由 Rust 写入
 * `SystemMediaTransportControls`，曲目信息与进度即出现在 Windows 媒体浮出窗、
 * 锁屏与蓝牙设备上；系统侧的播放 / 暂停 / 上一首 / 下一首按钮则通过
 * `smtc-button` 事件反向驱动播放器。
 *
 * 推送策略：
 * - 曲目、播放状态、时长变化 → 立即同步（这些变化用户能立刻感知）
 * - 进度变化 → 节流到每秒一次，避免频繁 IPC
 * - 同一时刻只允许一个同步请求在途，新的快照会覆盖待发送的旧快照，
 *   保证 Rust 侧最终收敛到最新状态且顺序不会错乱
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { watch, type WatchStopHandle } from "vue";
import type { UnifiedTrack } from "@/types/plugin";

/** Rust → 前端的按钮事件名（见 src-tauri/src/smtc.rs） */
const BUTTON_EVENT = "smtc-button";

/** 进度更新的最小间隔（毫秒） */
const POSITION_INTERVAL = 1_000;

export type SmtcStatus = "playing" | "paused" | "stopped";
export type SmtcButton = "play" | "pause" | "stop" | "next" | "previous";

/** 与 Rust 端 `SmtcSnapshot` 对应的快照（serde 转换为 camelCase） */
export interface SmtcSnapshot {
  /** 是否存在已加载的曲目；false 时隐藏系统媒体控件 */
  active: boolean;
  status: SmtcStatus;
  title: string;
  artist: string;
  album: string;
  cover: string | null;
  /** 播放位置（毫秒） */
  positionMs: number;
  /** 总时长（毫秒） */
  durationMs: number;
}

/** 服务需要的播放器能力（避免服务反向依赖 store 造成循环引用） */
export interface SmtcPlayer {
  readonly currentTrack: UnifiedTrack | null;
  readonly playing: boolean;
  readonly currentTime: number;
  readonly duration: number;
  togglePlay(): Promise<void>;
  next(): Promise<void>;
  prev(): Promise<void>;
}

/** 非 Tauri 环境（如浏览器里跑 `pnpm dev`）直接降级，不产生报错噪声 */
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** SMTC 是 Windows 专属能力，其他平台（Android/iOS/macOS/Linux）整体短路 */
const onWindows = typeof navigator !== "undefined" && /windows/i.test(navigator.userAgent);

let player: SmtcPlayer | null = null;
let enabled = false;
let started = false;
let stopWatch: WatchStopHandle | null = null;
let unlisten: Promise<() => void> | null = null;

let pending: SmtcSnapshot | null = null;
/** 最近一次进入发送流程的快照，用于判断变化是否“显著” */
let last: SmtcSnapshot | null = null;
let inFlight = false;
let timer: ReturnType<typeof setTimeout> | null = null;
let warned = false;

function inactiveSnapshot(): SmtcSnapshot {
  return {
    active: false,
    status: "stopped",
    title: "",
    artist: "",
    album: "",
    cover: null,
    positionMs: 0,
    durationMs: 0,
  };
}

function buildSnapshot(): SmtcSnapshot {
  const track = player?.currentTrack ?? null;
  if (!track) return inactiveSnapshot();

  const positionMs = Math.max(0, Math.round((player?.currentTime ?? 0) * 1000));
  const engineDurationMs = Math.round((player?.duration ?? 0) * 1000);
  return {
    active: true,
    status: player?.playing ? "playing" : "paused",
    title: track.name,
    artist: (track.artists ?? []).join(" / "),
    album: track.album ?? "",
    cover: track.cover?.trim() ? track.cover : null,
    positionMs,
    durationMs: engineDurationMs > 0 ? engineDurationMs : (track.duration ?? 0),
  };
}

/** 是否需要立即同步（曲目 / 状态 / 时长变化，而非单纯进度推进） */
function significant(previous: SmtcSnapshot, next: SmtcSnapshot): boolean {
  return (
    previous.active !== next.active ||
    previous.status !== next.status ||
    previous.title !== next.title ||
    previous.artist !== next.artist ||
    previous.album !== next.album ||
    previous.cover !== next.cover ||
    previous.durationMs !== next.durationMs
  );
}

function queue(snapshot: SmtcSnapshot): void {
  if (!enabled) return;
  const previous = pending ?? last;
  pending = snapshot;
  if (!previous || significant(previous, snapshot)) flush();
  else schedule();
}

function schedule(): void {
  if (timer !== null) return;
  timer = setTimeout(() => {
    timer = null;
    flush();
  }, POSITION_INTERVAL);
}

function flush(): void {
  if (timer !== null) {
    clearTimeout(timer);
    timer = null;
  }
  if (inFlight || !pending) return;

  const snapshot = pending;
  pending = null;
  last = snapshot;
  inFlight = true;
  invoke("smtc_sync", { snapshot })
    .catch((error) => {
      if (warned) return;
      warned = true;
      console.warn("[smtc] 同步系统媒体控件失败：", error);
    })
    .finally(() => {
      inFlight = false;
      // 在途期间积累了新快照（含禁用时的隐藏快照），继续送最新的一份
      if (pending) flush();
    });
}

function handleButton(action: string): void {
  const target = player;
  if (!target) return;
  switch (action as SmtcButton) {
    case "play":
      if (!target.playing) void target.togglePlay();
      break;
    case "pause":
    case "stop":
      if (target.playing) void target.togglePlay();
      break;
    case "next":
      void target.next();
      break;
    case "previous":
      void target.prev();
      break;
  }
}

/** 启动桥接：监听播放器状态并注册系统按钮回调（应用启动时调用一次） */
export function initSmtc(target: SmtcPlayer): void {
  if (started || !inTauri || !onWindows) return;
  started = true;
  player = target;

  unlisten = listen<string>(BUTTON_EVENT, (event) => handleButton(event.payload));
  unlisten.catch(() => {
    unlisten = null;
  });

  stopWatch = watch(() => buildSnapshot(), (snapshot) => queue(snapshot), { immediate: true });
}

/** 开关系统媒体控件（设置项） */
export function setSmtcEnabled(value: boolean): void {
  if (!onWindows) return;
  const changed = enabled !== value;
  enabled = value;
  if (!started || !inTauri || !changed) return;

  if (value) {
    queue(buildSnapshot());
  } else {
    // 立即撤下系统媒体控件，避免残留上一首的曲目信息
    pending = inactiveSnapshot();
    flush();
  }
}

/** 卸载桥接（应用退出时调用） */
export function disposeSmtc(): void {
  stopWatch?.();
  stopWatch = null;
  const pendingUnlisten = unlisten;
  unlisten = null;
  if (pendingUnlisten) void pendingUnlisten.then((fn) => fn()).catch(() => undefined);
  if (timer !== null) {
    clearTimeout(timer);
    timer = null;
  }
  started = false;
  player = null;
}
