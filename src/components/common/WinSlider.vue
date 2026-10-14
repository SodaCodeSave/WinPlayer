<script setup lang="ts">
/**
 * WinUI 风格滑块：悬停/拖动时显示圆形滑块头（对齐 WinUI Slider 交互）
 */
import { computed, ref } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    /** 拖动期间是否暂停向父组件同步（拖动结束后恢复） */
    deferSync?: boolean;
  }>(),
  {
    min: 0,
    max: 1,
    step: 0.01,
    disabled: false,
    deferSync: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: number];
  /** 拖动开始 / 结束（用于进度条拖动期间暂停引擎 timeupdate 覆盖） */
  dragstart: [];
  dragend: [value: number];
}>();

const trackRef = ref<HTMLElement | null>(null);
const dragging = ref(false);
const hoverValue = ref<number | null>(null);

const percent = computed(() => {
  const span = props.max - props.min;
  if (span <= 0) return 0;
  return Math.min(100, Math.max(0, ((props.modelValue - props.min) / span) * 100));
});

const active = computed(() => dragging.value || hoverValue.value !== null);

function valueFromEvent(event: PointerEvent): number {
  const el = trackRef.value;
  if (!el) return props.min;
  const rect = el.getBoundingClientRect();
  const ratio = Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width));
  const raw = props.min + ratio * (props.max - props.min);
  const stepped = props.step > 0 ? Math.round(raw / props.step) * props.step : raw;
  return Math.min(props.max, Math.max(props.min, stepped));
}

function onPointerEnter(event: PointerEvent) {
  if (props.disabled) return;
  hoverValue.value = valueFromEvent(event);
}

function onPointerMove(event: PointerEvent) {
  if (props.disabled) return;
  hoverValue.value = valueFromEvent(event);
  if (dragging.value) {
    const value = valueFromEvent(event);
    if (!props.deferSync) emit("update:modelValue", value);
    else previewValue.value = value;
  }
}

function onPointerDown(event: PointerEvent) {
  if (props.disabled) return;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  dragging.value = true;
  emit("dragstart");
  const value = valueFromEvent(event);
  previewValue.value = value;
  if (!props.deferSync) emit("update:modelValue", value);
}

function onPointerUp(event: PointerEvent) {
  if (!dragging.value) return;
  (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
  dragging.value = false;
  const value = valueFromEvent(event);
  previewValue.value = null;
  emit("update:modelValue", value);
  emit("dragend", value);
}

function onPointerLeave() {
  if (!dragging.value) hoverValue.value = null;
}

/** deferSync 模式下（如进度条拖动）的预览值 */
const previewValue = ref<number | null>(null);

const fillPercent = computed(() => {
  if (previewValue.value !== null && props.max > props.min) {
    return Math.min(100, Math.max(0, ((previewValue.value - props.min) / (props.max - props.min)) * 100));
  }
  return percent.value;
});
</script>

<template>
  <div
    ref="trackRef"
    class="win-slider"
    :class="{ disabled, dragging, active }"
    @pointerenter="onPointerEnter"
    @pointermove="onPointerMove"
    @pointerleave="onPointerLeave"
    @pointerdown="onPointerDown"
    @pointerup="onPointerUp"
  >
    <div class="slider-track">
      <div class="slider-fill" :style="{ width: fillPercent + '%' }" />
      <div class="slider-thumb" :style="{ left: fillPercent + '%' }" />
    </div>
  </div>
</template>

<style scoped>
.win-slider {
  position: relative;
  height: 20px;
  display: flex;
  align-items: center;
  cursor: pointer;
  touch-action: none;
}

.win-slider.disabled {
  cursor: default;
  opacity: 0.4;
}

.slider-track {
  position: relative;
  width: 100%;
  height: 4px;
  border-radius: 2px;
  background: var(--wp-control-stroke-secondary);
  overflow: visible;
}

.slider-fill {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  border-radius: 2px;
  background: var(--wp-accent);
  transition: none;
}

.slider-thumb {
  position: absolute;
  top: 50%;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--wp-accent);
  transform: translate(-50%, -50%) scale(0);
  transition: transform var(--duration-fast) var(--ease-decelerate);
  box-shadow: 0 0 0 0 rgba(0, 0, 0, 0.05);
  pointer-events: none;
}

.win-slider.active .slider-thumb,
.win-slider.dragging .slider-thumb {
  transform: translate(-50%, -50%) scale(1);
}

.win-slider.active .slider-track {
  background: var(--wp-divider);
}
</style>
