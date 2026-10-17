/**
 * 把 src-tauri/icons/android 里由 `tauri icon` 生成的启动图标同步进 gen/android 工程。
 *
 * 背景：Tauri 的 android 模板自带一套「默认启动图标」，`tauri android init` 只负责
 * 解压模板，不会用 src-tauri/icons/android 覆盖它（只有 `tauri icon` 在 gen/android
 * 已存在时才会写入 app/src/main/res）。因此在 `tauri android init` 之后、构建之前
 * 必须执行本脚本，否则 APK 始终显示 Tauri 默认图标。
 *
 * 用法：pnpm android:icons   （或 node scripts/apply-android-icons.mjs）
 */
import { cpSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const srcDir = join(root, "src-tauri", "icons", "android");
const resDir = join(root, "src-tauri", "gen", "android", "app", "src", "main", "res");

if (!existsSync(resDir)) {
  console.error(`[android-icons] 未找到 ${resDir}`);
  console.error("[android-icons] 请先执行：pnpm tauri android init");
  process.exit(1);
}

const relPaths = [
  "mipmap-anydpi-v26/ic_launcher.xml",
  "values/ic_launcher_background.xml",
];
for (const dpi of ["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"]) {
  for (const name of ["ic_launcher.png", "ic_launcher_round.png", "ic_launcher_foreground.png"]) {
    relPaths.push(`mipmap-${dpi}/${name}`);
  }
}

const missing = [];
let copied = 0;
for (const rel of relPaths) {
  const from = join(srcDir, rel);
  if (!existsSync(from)) {
    missing.push(rel);
    continue;
  }
  const to = join(resDir, rel);
  mkdirSync(dirname(to), { recursive: true });
  cpSync(from, to);
  copied += 1;
}

if (missing.length > 0) {
  console.warn(`[android-icons] 源文件缺失，已跳过 ${missing.length} 项：`);
  for (const rel of missing) console.warn(`  - src-tauri/icons/android/${rel}`);
  console.warn("[android-icons] 可执行 `pnpm tauri icon <源图>` 重新生成这些图标。");
}

console.log(`[android-icons] 已同步 ${copied} 个图标到 ${resDir}`);
