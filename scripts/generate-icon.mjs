/**
 * 生成 WinPlayer 应用图标源文件（1024x1024 PNG，RGBA）
 * 纯 Node 实现：蓝色渐变圆角方块 + 白色双音符，4x4 超采样抗锯齿
 * 用法：node scripts/generate-icon.mjs
 */
import { deflateSync, crc32 } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SIZE = 1024;
const SS = 4; // 超采样倍率

/** 场景：返回 [r,g,b,a]（0-255），坐标为超采样后的浮点坐标 */
function sampleScene(x, y) {
  // 背景：圆角矩形 + 渐变
  const radius = 210;
  const inside = roundedRectInside(x, y, SIZE, SIZE, radius);
  if (!inside) return [0, 0, 0, 0];

  const t = Math.min(1, Math.max(0, (x / SIZE + y / SIZE) / 2));
  // #005FB8 -> #4CC2FF
  const r = Math.round(0 + (76 - 0) * t);
  const g = Math.round(95 + (194 - 95) * t);
  const b = Math.round(184 + (255 - 184) * t);

  // 白色音符覆盖率
  const note = noteCoverage(x, y);
  if (note > 0) {
    return [Math.round(r + (255 - r) * note), Math.round(g + (255 - g) * note), Math.round(b + (255 - b) * note), 255];
  }
  return [r, g, b, 255];
}

function roundedRectInside(x, y, w, h, r) {
  if (x < 0 || y < 0 || x > w || y > h) return false;
  const cx = x < r ? r : x > w - r ? w - r : x;
  const cy = y < r ? r : y > h - r ? h - r : y;
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= r * r;
}

function noteCoverage(x, y) {
  let cov = 0;
  // 符头（椭圆）
  cov = Math.max(cov, ellipseCoverage(x, y, 331, 682, 95, 70));
  cov = Math.max(cov, ellipseCoverage(x, y, 651, 598, 95, 70));
  // 符干（矩形）
  cov = Math.max(cov, rectCoverage(x, y, 400, 330, 438, 770));
  cov = Math.max(cov, rectCoverage(x, y, 720, 246, 758, 686));
  // 符尾横梁（四边形）
  cov = Math.max(
    cov,
    polygonCoverage(x, y, [
      [400, 330],
      [758, 246],
      [758, 336],
      [400, 420],
    ]),
  );
  return cov;
}

function ellipseCoverage(x, y, cx, cy, rx, ry) {
  const dx = (x - cx) / rx;
  const dy = (y - cy) / ry;
  return dx * dx + dy * dy <= 1 ? 1 : 0;
}

function rectCoverage(x, y, x1, y1, x2, y2) {
  return x >= x1 && x <= x2 && y >= y1 && y <= y2 ? 1 : 0;
}

function polygonCoverage(x, y, points) {
  let inside = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i++) {
    const [xi, yi] = points[i];
    const [xj, yj] = points[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) {
      inside = !inside;
    }
  }
  return inside ? 1 : 0;
}

/** 渲染像素数据（RGBA） */
const pixels = Buffer.alloc(SIZE * SIZE * 4);
for (let py = 0; py < SIZE; py++) {
  for (let px = 0; px < SIZE; px++) {
    let r = 0;
    let g = 0;
    let b = 0;
    let a = 0;
    for (let sy = 0; sy < SS; sy++) {
      for (let sx = 0; sx < SS; sx++) {
        const [sr, sg, sb, sa] = sampleScene(px + (sx + 0.5) / SS, py + (sy + 0.5) / SS);
        r += sr;
        g += sg;
        b += sb;
        a += sa;
      }
    }
    const n = SS * SS;
    const offset = (py * SIZE + px) * 4;
    pixels[offset] = Math.round(r / n);
    pixels[offset + 1] = Math.round(g / n);
    pixels[offset + 2] = Math.round(b / n);
    pixels[offset + 3] = Math.round(a / n);
  }
}

/** 扫描线压缩每行（filter type 0） */
const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
for (let y = 0; y < SIZE; y++) {
  raw[y * (SIZE * 4 + 1)] = 0;
  pixels.copy(raw, y * (SIZE * 4 + 1) + 1, y * SIZE * 4, (y + 1) * SIZE * 4);
}
const idat = deflateSync(raw, { level: 9 });

/** PNG 组装 */
function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const typeBuffer = Buffer.from(type, "ascii");
  const crcBuffer = Buffer.alloc(4);
  crcBuffer.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])) >>> 0);
  return Buffer.concat([length, typeBuffer, data, crcBuffer]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", idat),
  chunk("IEND", Buffer.alloc(0)),
]);

const outDir = join(dirname(fileURLToPath(import.meta.url)), "..", "build-assets");
mkdirSync(outDir, { recursive: true });
const outPath = join(outDir, "app-icon.png");
writeFileSync(outPath, png);
console.log(`Icon source saved: ${outPath} (${png.length} bytes)`);
