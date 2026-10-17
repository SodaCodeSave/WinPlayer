<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "@/services/toast";
import { clearDataCache, dataCacheSize } from "@/services/cache";

/** 缓存占用（进入本页与清理后重新统计） */
const imageCount = ref(0);
const imageBytes = ref(0);
const dataBytes = ref(0);
const cacheLoading = ref(false);

interface CacheStats {
  imageCount: number;
  imageBytes: number;
}

async function refreshCacheStats(): Promise<void> {
  cacheLoading.value = true;
  try {
    const stats = await invoke<CacheStats>("cache_stats");
    imageCount.value = stats.imageCount;
    imageBytes.value = stats.imageBytes;
    dataBytes.value = await dataCacheSize();
  } catch {
    /* 统计失败时保持原值 */
  } finally {
    cacheLoading.value = false;
  }
}

async function clearCache(): Promise<void> {
  cacheLoading.value = true;
  try {
    await invoke<CacheStats>("cache_clear");
    await clearDataCache();
    toast("缓存已清除", "success");
    await refreshCacheStats();
  } catch (error) {
    toast(`清除缓存失败: ${error}`, "error");
  } finally {
    cacheLoading.value = false;
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value < 10 && unit > 0 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

const cacheSummary = computed(
  () =>
    `封面图片 ${formatBytes(imageBytes.value)}（${imageCount.value} 张） · 页面数据 ${formatBytes(dataBytes.value)}`,
);

onMounted(() => {
  void refreshCacheStats();
});
</script>

<template>
  <div>
    <!-- 存储与缓存 -->
    <section class="settings-section">
      <h2 class="settings-section-title">存储与缓存</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">清除缓存</div>
          <div class="card-description">
            当前占用：{{ cacheSummary }}。清除后封面图片和主页/歌单页数据将在下次访问时重新下载。
          </div>
        </div>
        <button class="wp-standard-button small danger" :disabled="cacheLoading" @click="clearCache">
          清除缓存
        </button>
      </div>
    </section>
  </div>
</template>
