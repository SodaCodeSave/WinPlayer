/**
 * 插件宿主：管理每个插件的沙箱 Worker 生命周期，代理沙箱发起的 RPC 请求
 * （网络 → Rust http_fetch；存储 → Rust plugin_storage_*）。
 */

import { invoke } from "@tauri-apps/api/core";
import { httpFetch } from "@/services/proxy";
import type { PluginEntry, PluginRequest, PluginResponse } from "@/types/plugin";
import type { HostMessage, WorkerMessage } from "./protocol";

const INIT_TIMEOUT_MS = 20_000;
const INVOKE_TIMEOUT_MS = 45_000;

interface PendingInvoke {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timer: number;
}

/**
 * postMessage 要求可结构化克隆的数据；Vue 响应式对象（Proxy）不可克隆，
 * 统一经 JSON 深拷贝转为纯数据后再发给 Worker。
 */
function toCloneable<T>(value: T): T {
  if (value === undefined || value === null) return value;
  return JSON.parse(JSON.stringify(value)) as T;
}

async function proxyHttp(payload: PluginRequest): Promise<PluginResponse> {
  const result = await httpFetch({
    url: payload.url,
    method: payload.method ?? "GET",
    headers: payload.headers ?? {},
    body: payload.body,
    responseType: payload.responseType === "base64" ? "base64" : "text",
    timeoutMs: payload.timeout ?? 15_000,
  });

  const response: PluginResponse = {
    status: result.status,
    ok: result.ok,
    headers: result.headers ?? {},
  };

  if (payload.responseType === "base64") {
    response.base64 = result.bodyBase64;
    return response;
  }

  response.text = result.body ?? "";
  if (payload.responseType === "json" || payload.responseType === undefined || payload.responseType === "auto") {
    try {
      response.data = JSON.parse(response.text);
    } catch {
      /* 非 JSON，保留纯文本 */
    }
  }
  return response;
}

/** 一个已加载插件的沙箱实例 */
export class PluginHost {
  readonly entry: PluginEntry;

  private readonly worker: Worker;
  private readonly pending = new Map<number, PendingInvoke>();
  private initResolve: (() => void) | null = null;
  private initReject: ((error: Error) => void) | null = null;
  private initialized = false;
  private disposed = false;
  private invokeId = 0;

  constructor(entry: PluginEntry, code: string, settings: Record<string, unknown>) {
    this.entry = entry;
    this.worker = new Worker(new URL("./sandbox-worker.ts", import.meta.url), {
      type: "module",
      name: `winplayer-plugin-${entry.id}`,
    });
    this.worker.addEventListener("message", this.onMessage);
    this.worker.addEventListener("error", this.onWorkerError);

    const initPromise = new Promise<void>((resolve, reject) => {
      this.initResolve = resolve;
      this.initReject = reject;
    });

    const initTimer = window.setTimeout(() => {
      this.initReject?.(new Error(`插件 ${entry.manifest.name} 初始化超时`));
    }, INIT_TIMEOUT_MS);
    void initPromise.finally(() => window.clearTimeout(initTimer));

    this.post({ type: "init", code, pluginId: entry.id, settings: toCloneable(settings) });
    // 初始化结果在 onMessage 中兑现
    this.ready = initPromise;
  }

  readonly ready: Promise<void>;

  /** 插件是否具备某能力 */
  supports(capability: string): boolean {
    return this.entry.manifest.capabilities?.includes(capability as never) ?? false;
  }

  /** 调用插件方法；插件未实现该方法时返回 null */
  async invoke<T>(method: string, args: Record<string, unknown> = {}): Promise<T | null> {
    if (this.disposed) throw new Error("插件已卸载");
    await this.ready;

    const id = ++this.invokeId;
    return new Promise<T | null>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`插件方法 ${method} 调用超时`));
      }, INVOKE_TIMEOUT_MS);
      this.pending.set(id, {
        resolve: (value) => resolve((value ?? null) as T | null),
        reject,
        timer,
      });
      this.post({ type: "invoke", id, method, args: toCloneable(args) });
    });
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.post({ type: "dispose" });
    this.worker.removeEventListener("message", this.onMessage);
    this.worker.removeEventListener("error", this.onWorkerError);
    this.worker.terminate();
    for (const pending of this.pending.values()) {
      window.clearTimeout(pending.timer);
      pending.reject(new Error("插件已卸载"));
    }
    this.pending.clear();
  }

  private post(message: HostMessage): void {
    this.worker.postMessage(message);
  }

  private onWorkerError = (event: ErrorEvent): void => {
    const error = new Error(event.message || "插件沙箱发生错误");
    if (!this.initialized) {
      this.initReject?.(error);
      this.initialized = true;
    }
    for (const pending of this.pending.values()) {
      window.clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  };

  private onMessage = (event: MessageEvent<WorkerMessage>): void => {
    void this.handleMessage(event.data);
  };

  private async handleMessage(message: WorkerMessage): Promise<void> {
    switch (message.type) {
      case "result": {
        if (message.id === 0) {
          // 初始化结果
          if (!this.initialized) {
            this.initialized = true;
            if (message.ok) this.initResolve?.();
            else this.initReject?.(new Error(message.error ?? "插件初始化失败"));
          }
          return;
        }
        const pending = this.pending.get(message.id);
        if (pending) {
          this.pending.delete(message.id);
          window.clearTimeout(pending.timer);
          if (message.ok) pending.resolve(message.value);
          else pending.reject(new Error(message.error ?? "插件调用失败"));
        }
        return;
      }
      case "rpc-request": {
        try {
          const value = await this.handleRpc(message.payload);
          this.post({ type: "rpc-response", id: message.id, ok: true, value });
        } catch (error) {
          this.post({
            type: "rpc-response",
            id: message.id,
            ok: false,
            error: error instanceof Error ? error.message : String(error),
          });
        }
        return;
      }
      case "log": {
        const printer =
          message.level === "error" ? console.error : message.level === "warn" ? console.warn : console.log;
        printer(`[插件 ${this.entry.id}]`, ...message.args);
        return;
      }
    }
  }

  /** 沙箱内部请求路由：存储类走 Rust 落盘，其余走 HTTP 代理 */
  private async handleRpc(payload: PluginRequest): Promise<PluginResponse> {
    if (payload.url === "plugin-internal://storage/get") {
      const value = await invoke<Record<string, unknown>>("plugin_storage_get", { id: this.entry.id });
      return { status: 200, ok: true, headers: {}, text: JSON.stringify(value ?? {}), data: value };
    }
    if (payload.url === "plugin-internal://storage/set") {
      let parsed: unknown = {};
      try {
        parsed = JSON.parse(payload.body ?? "{}");
      } catch {
        parsed = {};
      }
      await invoke("plugin_storage_set", { id: this.entry.id, value: parsed });
      return { status: 200, ok: true, headers: {}, text: "ok" };
    }
    return proxyHttp(payload);
  }
}
