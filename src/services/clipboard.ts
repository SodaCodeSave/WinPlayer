/** 剪贴板工具：复制并给出 Toast 反馈 */

import { toast } from "@/services/toast";

/** 复制文本到系统剪贴板，成功/失败均有 Toast 提示 */
export async function copyText(text: string, label = "内容"): Promise<void> {
  const value = (text ?? "").trim();
  if (!value) {
    toast(`没有可复制的${label}`, "info");
    return;
  }
  try {
    await navigator.clipboard.writeText(value);
    toast(`已复制${label}`, "success");
  } catch {
    // WebView 剪贴板 API 不可用时的兜底方案
    const textarea = document.createElement("textarea");
    textarea.value = value;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    try {
      if (document.execCommand("copy")) {
        toast(`已复制${label}`, "success");
      } else {
        toast("复制失败", "error");
      }
    } catch {
      toast("复制失败", "error");
    } finally {
      textarea.remove();
    }
  }
}
