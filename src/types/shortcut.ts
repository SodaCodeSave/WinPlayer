/**
 * 键盘快捷键的类型与默认绑定定义。
 *
 * 应用内快捷键仅在窗口处于前台（渲染进程能收到键盘事件）时生效，不注册系统级全局热键。
 * 空格「播放 / 暂停」为内置、不可设置的快捷键，不在下面的可配置动作列表中。
 */

/** 一次按键绑定（与 KeyboardEvent 解耦，便于持久化与展示） */
export interface ShortcutBinding {
  /** 归一化后的 KeyboardEvent.key（小写；空格记为 "space"） */
  key: string;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
}

/** 可配置的快捷键动作 ID */
export type ShortcutActionId = "prev" | "next" | "volumeUp" | "volumeDown" | "cycleMode";

/** 动作定义 */
export interface ShortcutActionMeta {
  id: ShortcutActionId;
  /** 设置页显示名 */
  label: string;
  /** 设置页说明 */
  description: string;
  /** 默认绑定；null 表示默认不设置 */
  defaultBinding: ShortcutBinding | null;
}

/** 全部可配置动作（数组顺序即冲突优先级：靠前者先匹配，也决定设置页展示顺序） */
export const SHORTCUT_ACTIONS: readonly ShortcutActionMeta[] = [
  {
    id: "prev",
    label: "上一首",
    description: "切换到播放队列中的上一首",
    defaultBinding: { key: "arrowleft", ctrl: true, shift: false, alt: false, meta: false },
  },
  {
    id: "next",
    label: "下一首",
    description: "切换到播放队列中的下一首",
    defaultBinding: { key: "arrowright", ctrl: true, shift: false, alt: false, meta: false },
  },
  {
    id: "volumeUp",
    label: "音量加",
    description: "提高播放音量（每次 5%）",
    defaultBinding: null,
  },
  {
    id: "volumeDown",
    label: "音量减",
    description: "降低播放音量（每次 5%）",
    defaultBinding: null,
  },
  {
    id: "cycleMode",
    label: "播放模式切换",
    description: "在顺序播放 / 列表循环 / 单曲循环 / 随机播放之间切换",
    defaultBinding: null,
  },
];

/** 音量加 / 减每次调整的步长（0~1） */
export const VOLUME_STEP = 0.05;

/** 动作 ID → 绑定（null 表示未绑定） */
export type ShortcutMap = Record<ShortcutActionId, ShortcutBinding | null>;

/** 生成一份默认绑定表（深拷贝，避免外部改动默认值） */
export function defaultShortcutMap(): ShortcutMap {
  const map = {} as ShortcutMap;
  for (const action of SHORTCUT_ACTIONS) {
    map[action.id] = action.defaultBinding ? { ...action.defaultBinding } : null;
  }
  return map;
}
