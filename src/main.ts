import { createApp } from "vue";
import { createPinia } from "pinia";
import { WinUIOnWeb } from "winuionweb";
import "winuionweb/style.css";
import "@/styles/tokens.css";
import "@/styles/global.css";
import "@/styles/mobile.css";
import App from "./App.vue";
import router from "./router";
import { installScrollMemory } from "@/services/scrollMemory";
import { useIsMobile } from "@/composables/useIsMobile";

const app = createApp(App);

app.use(createPinia());
app.use(router);

// 返回时恢复滚动位置（仅移动端；桌面端行为保持不变）
if (useIsMobile()) installScrollMemory(router);
app.use(WinUIOnWeb, { locale: "zh-CN" });

app.mount("#app");
