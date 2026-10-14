import type { PluginRequest, PluginResponse } from "@/types/plugin";

/**
 * 宿主（主线程）与插件沙箱（Web Worker）之间的 RPC 协议
 */

/** 宿主 → Worker */
export type HostMessage =
  | { type: "init"; code: string; pluginId: string; settings: Record<string, unknown> }
  | { type: "invoke"; id: number; method: string; args: Record<string, unknown> }
  | { type: "rpc-response"; id: number; ok: boolean; value?: unknown; error?: string }
  | { type: "dispose" };

/** Worker → 宿主 */
export type WorkerMessage =
  | { type: "result"; id: number; ok: boolean; value?: unknown; error?: string }
  | { type: "rpc-request"; id: number; payload: PluginRequest }
  | { type: "log"; level: "log" | "warn" | "error"; args: unknown[] };

/** 代理请求的响应载荷 */
export type RpcResponseValue = PluginResponse;
