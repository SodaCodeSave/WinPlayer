/** 键盘快捷键设置：可配置动作的绑定与全局按键处理（持久化于 SQLite settings 表） */

import { defineStore } from "pinia";
import { ref } from "vue";
import { getSetting, setSetting } from "@/services/db";
import {
  isEditableTarget,
  isInteractiveTarget,
  isPlainSpace,
  isShortcutBinding,
  matchesBinding,
} from "@/services/shortcuts";
import {
  SHORTCUT_ACTIONS,
  VOLUME_STEP,
  defaultShortcutMap,
  type ShortcutActionId,
  type ShortcutBinding,
  type ShortcutMap,
} from "@/types/shortcut";
import { usePlayerStore } from "./player";

/** 设置表键名 */
const SETTING_KEY = "settings.shortcuts";

export const useShortcutsStore = defineStore("shortcuts", () => {
  const bindings = ref<ShortcutMap>(defaultShortcutMap());
  /** 是否正在录制：录制期间全局监听让位于设置页的录制控件 */
  const capturing = ref(false);

  /* ------------------------------ 持久化 ------------------------------ */

  async function load(): Promise<void> {
    const saved = await getSetting<Partial<Record<ShortcutActionId, unknown>>>(SETTING_KEY, {});
    const map = defaultShortcutMap();
    for (const action of SHORTCUT_ACTIONS) {
      if (!(action.id in saved)) continue;
      const value = saved[action.id];
      // null 表示用户显式清除；非法值回退到默认绑定
      if (value === null) map[action.id] = null;
      else if (isShortcutBinding(value)) map[action.id] = value;
    }
    bindings.value = map;
  }

  async function persist(): Promise<void> {
    await setSetting(SETTING_KEY, bindings.value);
  }

  /** 设置某动作的绑定 */
  async function setBinding(id: ShortcutActionId, binding: ShortcutBinding): Promise<void> {
    bindings.value = { ...bindings.value, [id]: binding };
    await persist();
  }

  /** 清除某动作的绑定 */
  async function clearBinding(id: ShortcutActionId): Promise<void> {
    if (bindings.value[id] === null) return;
    bindings.value = { ...bindings.value, [id]: null };
    await persist();
  }

  /** 全部恢复为默认绑定 */
  async function resetToDefaults(): Promise<void> {
    bindings.value = defaultShortcutMap();
    await persist();
  }

  /** 切换录制态（由设置页录制控件调用） */
  function setCapturing(value: boolean): void {
    capturing.value = value;
  }

  /* ------------------------------ 按键处理 ------------------------------ */

  /** 命中绑定则返回动作 ID；数组顺序即优先级，允许冲突时先注册者先匹配 */
  function actionForEvent(event: KeyboardEvent): ShortcutActionId | null {
    for (const action of SHORTCUT_ACTIONS) {
      const binding = bindings.value[action.id];
      if (binding && matchesBinding(event, binding)) return action.id;
    }
    return null;
  }

  /** 执行动作 */
  function runAction(action: ShortcutActionId): void {
    const player = usePlayerStore();
    switch (action) {
      case "prev":
        void player.prev();
        break;
      case "next":
        void player.next();
        break;
      case "volumeUp":
        void player.setVolume(player.volume + VOLUME_STEP);
        break;
      case "volumeDown":
        void player.setVolume(player.volume - VOLUME_STEP);
        break;
      case "cycleMode":
        player.cycleMode();
        break;
    }
  }

  /** 全局键盘处理（由 ShortcutHost 在 window 上挂载） */
  function handleKeydown(event: KeyboardEvent): void {
    // 录制中：交给设置页的录制控件处理
    if (capturing.value) return;
    // 可编辑区域：不拦截，保留输入与光标移动等原生行为
    if (isEditableTarget(event.target)) return;

    const action = actionForEvent(event);
    if (action) {
      // 音量允许长按连续调整，其余动作忽略自动重复以免连续切歌 / 切模式
      if (event.repeat && action !== "volumeUp" && action !== "volumeDown") return;
      event.preventDefault();
      runAction(action);
      return;
    }

    // 内置、不可设置的快捷键：空格播放 / 暂停（仅窗口在前台时生效）
    if (isPlainSpace(event)) {
      if (event.repeat) return;
      // 焦点在按钮等控件上时保留其原生空格行为（如激活按钮）
      if (isInteractiveTarget(event.target)) return;
      event.preventDefault();
      void usePlayerStore().togglePlay();
    }
  }

  return {
    bindings,
    capturing,
    load,
    setBinding,
    clearBinding,
    resetToDefaults,
    setCapturing,
    actionForEvent,
    handleKeydown,
  };
});
