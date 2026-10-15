/**
 * 页面数据缓存（主页推荐、插件歌单详情等）。
 *
 * 复用 SQLite settings 表作为 KV 存储，key 以 `cache.data.` 前缀隔离，
 * 与真正的用户设置互不干扰。每个条目附带 savedAt 时间戳，供界面提示新旧。
 */

import { getDb, getSetting, setSetting } from "@/services/db";

interface CacheEntry<T> {
  savedAt: number;
  data: T;
}

const KEY_PREFIX = "cache.data.";

export interface CachedData<T> {
  savedAt: number;
  data: T;
}

/** 读取缓存数据，不存在或损坏时返回 null（读取失败静默降级为无缓存） */
export async function getCachedData<T>(key: string): Promise<CachedData<T> | null> {
  try {
    const entry = await getSetting<CacheEntry<T> | null>(KEY_PREFIX + key, null);
    if (!entry || typeof entry.savedAt !== "number") return null;
    return { savedAt: entry.savedAt, data: entry.data };
  } catch {
    return null;
  }
}

/** 写入缓存数据（写入失败不影响主流程） */
export async function setCachedData<T>(key: string, data: T): Promise<void> {
  try {
    await setSetting(KEY_PREFIX + key, { savedAt: Date.now(), data });
  } catch {
    /* 缓存写失败不致命 */
  }
}

/** 清空全部页面数据缓存（图片缓存由 Rust 端 cache_clear 命令清理） */
export async function clearDataCache(): Promise<void> {
  const db = await getDb();
  await db.execute(`delete from settings where key like '${KEY_PREFIX}%'`);
}

/** 统计页面数据缓存占用（字节） */
export async function dataCacheSize(): Promise<number> {
  try {
    const db = await getDb();
    const rows = await db.select<Array<{ total: number | null }>>(
      "select sum(length(value)) as total from settings where key like 'cache.data.%'",
    );
    return Number(rows[0]?.total ?? 0);
  } catch {
    return 0;
  }
}
