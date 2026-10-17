/**
 * 桌面窗口材质（Mica / Acrylic）：仅 Windows 桌面端生效。
 *
 * 请求 Rust 端（window-vibrancy）给主窗口应用 DWM 材质，实际生效结果
 * 通过 `<html data-material="mica|acrylic|none">` 同步给 CSS——tokens.css
 * 在材质生效时把主背景换成半透明色，让系统材质透出来。
 */

import { invoke } from "@tauri-apps/api/core";

/** 窗口材质偏好：Mica（Win11）/ Acrylic / 关闭（不透明纯色背景） */
export type WindowMaterial = "mica" | "acrylic" | "off";

const inTauri = "__TAURI_INTERNALS__" in window;
const onWindows = /windows/i.test(navigator.userAgent);

/** 按偏好应用窗口材质；非 Windows/非 Tauri 环境是 no-op（仍会清残留并标记 none） */
export async function applyWindowMaterial(pref: WindowMaterial): Promise<void> {
  const root = document.documentElement;
  if (!inTauri || !onWindows || pref === "off") {
    // Windows 上切到「关闭」时需通知 Rust 清掉可能残留的材质
    if (inTauri && onWindows) {
      await invoke("window_set_material", { kind: "none", dark: null }).catch(() => {});
    }
    root.dataset.material = "none";
    return;
  }
  const applied = await invoke<string>("window_set_material", { kind: pref, dark: null }).catch(
    () => "none",
  );
  root.dataset.material = applied;
}
