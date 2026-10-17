<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";

function openRepo() {
  void openUrl("https://github.com/Furry-Xiyi/WinUIonWeb");
}
</script>

<template>
  <div>
    <!-- 关于 -->
    <section class="settings-section">
      <h2 class="settings-section-title">关于</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">WinPlayer</div>
          <div class="card-description">版本 1.4.0 · 基于插件的桌面音乐播放器 · MIT License</div>
        </div>
      </div>

      <div class="settings-card column">
        <div class="card-content">
          <div class="card-header">开源组件声明</div>
          <div class="card-description attribution">
            <p>
              本软件的图形界面使用了开源组件库
              <a href="#" @click.prevent="openRepo">WinUIonWeb</a>
              （作者 Furry-Xiyi），该组件库以 <strong>GPL-3.0-only</strong> 协议发布，是对 Microsoft WinUI 的独立 Web 实现
            </p>
            <p>
              WinPlayer 本体源代码以 MIT 协议发布；由于分发物中包含 GPL-3.0
              组件，整个软件的分发须同时遵守 GPL-3.0 的相应义务。更多第三方协议信息见仓库中的
              THIRD_PARTY_NOTICES.md。
            </p>
            <p>
              其他依赖：Tauri（MIT/Apache-2.0）、Vue（MIT）。
            </p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.attribution p {
  margin: 0 0 var(--space-md) 0;
}

.attribution p:last-child {
  margin-bottom: 0;
}
</style>
