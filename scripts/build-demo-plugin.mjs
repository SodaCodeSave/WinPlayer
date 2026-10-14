/**
 * 将 plugins/demo-source 打包为单文件 JSON 插件 + 生成市场索引示例
 * 用法：node scripts/build-demo-plugin.mjs
 */
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pluginDir = join(root, "plugins", "demo-source");

const manifest = JSON.parse(readFileSync(join(pluginDir, "manifest.json"), "utf-8"));
const code = readFileSync(join(pluginDir, "main.js"), "utf-8");

const singleFile = { ...manifest, code };

const outDir = join(root, "plugins", "dist");
mkdirSync(outDir, { recursive: true });

const json = JSON.stringify(singleFile, null, 2);
const pluginOut = join(outDir, `demo-source-${manifest.version}.json`);
writeFileSync(pluginOut, json);
console.log(`Single-file plugin saved: ${pluginOut}`);

// 市场索引示例（假设单文件插件被托管在某个可访问的 URL 上）
const hostedUrl = "https://example.com/plugins/demo-source-1.0.0.json";
const marketIndex = {
  name: "WinPlayer 官方示例市场（占位）",
  plugins: [
    {
      id: manifest.id,
      name: manifest.name,
      version: manifest.version,
      description: manifest.description,
      author: manifest.author,
      url: hostedUrl,
      homepage: manifest.homepage,
    },
  ],
};

const marketOut = join(root, "docs", "market-index.example.json");
mkdirSync(join(root, "docs"), { recursive: true });
writeFileSync(marketOut, JSON.stringify(marketIndex, null, 2));
console.log(`Market index example saved: ${marketOut}`);
