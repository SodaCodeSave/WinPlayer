/**
 * 构建前确保 Fluent UI System Icons 字体存在（MIT 授权，字体不入库）。
 * - 已存在且大小正常时跳过，离线/重复构建零开销
 * - 可用环境变量 FLUENT_FONT_BASE_URL 覆盖下载源（网络受限时指向镜像）
 */
import { existsSync, mkdirSync, writeFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const fontDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "src",
  "assets",
  "fonts",
);
const baseUrl =
  process.env.FLUENT_FONT_BASE_URL ??
  "https://raw.githubusercontent.com/microsoft/fluentui-system-icons/main/fonts";

const fonts = ["FluentSystemIcons-Regular.ttf", "FluentSystemIcons-Filled.ttf"];

for (const name of fonts) {
  const dest = path.join(fontDir, name);
  if (existsSync(dest) && statSync(dest).size > 1_000_000) {
    console.log(`[fonts] ${name} 已存在，跳过下载`);
    continue;
  }
  mkdirSync(fontDir, { recursive: true });
  const url = `${baseUrl}/${name}`;
  console.log(`[fonts] 下载 ${url} …`);
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`[fonts] 下载失败 ${res.status} ${res.statusText}（可用 FLUENT_FONT_BASE_URL 指定镜像）`);
  }
  const buf = Buffer.from(await res.arrayBuffer());
  writeFileSync(dest, buf);
  console.log(`[fonts] ${name} 完成（${(buf.length / 1024 / 1024).toFixed(2)} MB）`);
}
