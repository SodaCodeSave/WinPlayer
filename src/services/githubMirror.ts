/**
 * GitHub 镜像源工具：URL 前缀拼接型（如 https://ghproxy.net/），
 * 仅对 GitHub 系域名生效，其他地址原样返回。
 */

const GITHUB_URL_PREFIXES = [
  "https://github.com/",
  "https://raw.githubusercontent.com/",
  "https://codeload.github.com/",
  "https://api.github.com/",
  "https://gist.githubusercontent.com/",
  "https://objects.githubusercontent.com/",
];

/** 归一化镜像前缀：去空白与结尾斜杠，空串返回空串 */
export function normalizeMirror(mirror: string): string {
  return mirror.trim().replace(/\/+$/, "");
}

export function isGithubUrl(url: string): boolean {
  return GITHUB_URL_PREFIXES.some((prefix) => url.startsWith(prefix));
}

/** GitHub 系地址拼接镜像前缀，其余原样返回 */
export function applyGithubMirror(url: string, mirror: string): string {
  const normalized = normalizeMirror(mirror);
  if (!normalized || !isGithubUrl(url)) return url;
  return `${normalized}/${url}`;
}

/** 以市场索引地址为基准解析插件条目里的相对资源路径（banner 等），绝对地址原样返回 */
export function resolveAssetUrl(base: string, path: string): string {
  try {
    return new URL(path, base).toString();
  } catch {
    return path;
  }
}
