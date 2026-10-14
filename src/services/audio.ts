/**
 * 音频引擎：HTML5 Audio 封装。
 *
 * 预留均衡器（EQ）扩展点：后续接入 Web Audio API 时，
 * 在 connectToOutput() 处插入 BiquadFilterNode 链即可，
 * 播放器上层（store / UI）无需改动。
 */

export interface AudioEngineEvents {
  timeupdate: (currentTime: number, duration: number) => void;
  durationchange: (duration: number) => void;
  ended: () => void;
  error: (message: string) => void;
  playing: () => void;
  paused: () => void;
  waiting: () => void;
}

type EventName = keyof AudioEngineEvents;

export class AudioEngine {
  private readonly audio: HTMLAudioElement;
  private readonly listeners = new Map<EventName, Set<(...args: never[]) => void>>();

  /** EQ 是否已启用（当前版本为直通，恒为 false） */
  readonly eqEnabled = false;

  constructor() {
    this.audio = new Audio();
    this.audio.preload = "auto";
    this.audio.addEventListener("timeupdate", () => {
      this.emit("timeupdate", this.audio.currentTime, this.duration);
    });
    this.audio.addEventListener("durationchange", () => {
      this.emit("durationchange", this.duration);
    });
    this.audio.addEventListener("ended", () => this.emit("ended"));
    this.audio.addEventListener("playing", () => this.emit("playing"));
    this.audio.addEventListener("pause", () => this.emit("paused"));
    this.audio.addEventListener("waiting", () => this.emit("waiting"));
    this.audio.addEventListener("error", () => {
      const code = this.audio.error?.code;
      this.emit("error", describeMediaError(code));
    });
  }

  get currentTime(): number {
    return this.audio.currentTime;
  }

  get duration(): number {
    return Number.isFinite(this.audio.duration) ? this.audio.duration : 0;
  }

  get paused(): boolean {
    return this.audio.paused;
  }

  get volume(): number {
    return this.audio.volume;
  }

  set volume(value: number) {
    this.audio.volume = Math.min(1, Math.max(0, value));
  }

  get muted(): boolean {
    return this.audio.muted;
  }

  set muted(value: boolean) {
    this.audio.muted = value;
  }

  /** 设置播放源并加载（不自动播放） */
  load(src: string): void {
    this.audio.src = src;
    this.audio.load();
  }

  async play(): Promise<void> {
    await this.audio.play();
  }

  pause(): void {
    this.audio.pause();
  }

  seek(seconds: number): void {
    if (Number.isFinite(seconds)) {
      this.audio.currentTime = Math.max(0, seconds);
    }
  }

  stop(): void {
    this.audio.pause();
    this.audio.removeAttribute("src");
    this.audio.load();
  }

  on<K extends EventName>(event: K, listener: AudioEngineEvents[K]): () => void {
    let set = this.listeners.get(event);
    if (!set) {
      set = new Set();
      this.listeners.set(event, set);
    }
    set.add(listener as (...args: never[]) => void);
    return () => {
      set?.delete(listener as (...args: never[]) => void);
    };
  }

  private emit<K extends EventName>(event: K, ...args: Parameters<AudioEngineEvents[K]>): void {
    const set = this.listeners.get(event);
    if (!set) return;
    for (const listener of set) {
      (listener as (...a: unknown[]) => void)(...args);
    }
  }

  // ---------------------------------------------------------------------------
  // 均衡器预留接口（当前版本不启用；接入 Web Audio 后在此维护 BiquadFilter 链）
  // ---------------------------------------------------------------------------
}

function describeMediaError(code: number | undefined): string {
  switch (code) {
    case MediaError.MEDIA_ERR_ABORTED:
      return "播放被中断";
    case MediaError.MEDIA_ERR_NETWORK:
      return "网络错误，无法加载音频";
    case MediaError.MEDIA_ERR_DECODE:
      return "音频解码失败";
    case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED:
      return "音频源不可用或格式不支持";
    default:
      return "播放出错";
  }
}

let engine: AudioEngine | null = null;

export function getAudioEngine(): AudioEngine {
  if (!engine) engine = new AudioEngine();
  return engine;
}
