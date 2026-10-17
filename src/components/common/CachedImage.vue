<script setup lang="ts">
/**
 * 缓存图片：三级降级加载
 *
 * 1. 先请求缓存协议地址（磁盘命中秒开，未命中由 Rust 下载压缩后返回）；
 * 2. 失败则回退显示远程原图，同时后台调用 image_cache_prefetch 预取，
 *    保证下次能命中缓存（以 no-referrer 请求，规避图床防盗链——如 B 站
 *    封面 CDN 对非 bilibili Referer 直接返回 403）；
 * 3. 远程也失败则不再渲染图片（父组件的占位图标不受影响时可直接忽略）。
 *
 * 传入的 src 为归一化层改写后的缓存协议地址（见 services/imageCache.ts）。
 */
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { isImageCacheEnabled } from "@/services/imageCache";

const props = defineProps<{ src?: string; alt?: string }>();

const PROTOCOL_PREFIX = "http://cachedimg.localhost/";

/** 从缓存协议地址里解出原始远程地址 */
function originalOf(url: string): string | null {
  if (!url.startsWith(PROTOCOL_PREFIX)) return null;
  const marker = "?src=";
  const idx = url.indexOf(marker);
  if (idx === -1) return null;
  try {
    return decodeURIComponent(url.slice(idx + marker.length));
  } catch {
    return null;
  }
}

type Stage = "cache" | "remote" | "none";
// 图片缓存被禁用时初始即走远程原图（如历史数据里已存有缓存协议地址也能解出原图）
const stage = ref<Stage>(isImageCacheEnabled() ? "cache" : "remote");

// 换图时重置降级进度；图片缓存被禁用时直接走远程原图
watch(
  () => props.src,
  () => {
    stage.value = isImageCacheEnabled() ? "cache" : "remote";
  },
);

const displaySrc = computed(() => {
  if (!props.src || stage.value === "none") return "";
  if (stage.value === "cache") return props.src;
  return originalOf(props.src) ?? props.src;
});

async function onError(): Promise<void> {
  if (stage.value === "cache") {
    stage.value = "remote";
    // 回退显示远程的同时预取缓存，下次打开就能走磁盘
    const remote = props.src ? originalOf(props.src) : null;
    if (remote) void invoke("image_cache_prefetch", { url: remote }).catch(() => {});
  } else if (stage.value === "remote") {
    stage.value = "none";
  }
}
</script>

<template>
  <img
    v-if="displaySrc"
    :src="displaySrc"
    :alt="alt ?? ''"
    loading="lazy"
    decoding="async"
    draggable="false"
    referrerpolicy="no-referrer"
    @error="onError"
  />
</template>
