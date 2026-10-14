/**
 * SQLite 数据访问（tauri-plugin-sql）。
 * 表结构由 src-tauri/migrations/001_init.sql 在应用首次启动时自动创建。
 */

import Database from "@tauri-apps/plugin-sql";

let db: Database | null = null;

export async function getDb(): Promise<Database> {
  if (!db) {
    db = await Database.load("sqlite:winplayer.db");
  }
  return db;
}

/** 通用 key-value 设置读取 */
export async function getSetting<T>(key: string, fallback: T): Promise<T> {
  const database = await getDb();
  const rows = await database.select<Array<{ value: string }>>(
    "select value from settings where key = $1",
    [key],
  );
  if (!rows.length) return fallback;
  try {
    return JSON.parse(rows[0].value) as T;
  } catch {
    return fallback;
  }
}

/** 通用 key-value 设置写入 */
export async function setSetting(key: string, value: unknown): Promise<void> {
  const database = await getDb();
  await database.execute(
    "insert into settings (key, value) values ($1, $2) on conflict(key) do update set value = $2",
    [key, JSON.stringify(value)],
  );
}
