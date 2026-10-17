<script setup lang="ts">
import { ToggleSwitch } from "winuionweb";
import { useSettingsStore } from "@/stores/settings";
import { useIsWindows } from "@/composables/useIsMobile";

const settings = useSettingsStore();
// SMTC 是 Windows 专属能力，其他平台隐藏该设置项
const isWindows = useIsWindows();

function onSmtcToggle(event: { IsOn: boolean }): void {
  void settings.setSmtcEnabled(Boolean(event?.IsOn));
}
</script>

<template>
  <div>
    <!-- 系统集成（SMTC 仅 Windows 提供） -->
    <section v-if="isWindows" class="settings-section">
      <h2 class="settings-section-title">系统集成</h2>
      <div class="settings-card">
        <div class="card-content">
          <div class="card-header">系统媒体传输控件（SMTC）</div>
          <div class="card-description">
            在 Windows 媒体浮出窗、锁屏与蓝牙设备上显示正在播放的曲目、封面与进度，并响应系统的播放 / 暂停 / 上一首 / 下一首按钮
          </div>
        </div>
        <ToggleSwitch :IsOn="settings.smtcEnabled" @Toggled="onSmtcToggle" />
      </div>
    </section>
  </div>
</template>
