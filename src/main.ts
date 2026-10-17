import { createApp } from "vue";
import { createPinia } from "pinia";
import { WinUIOnWeb } from "winuionweb";
import "winuionweb/style.css";
import "@/styles/tokens.css";
import "@/styles/global.css";
import "@/styles/mobile.css";
import App from "./App.vue";
import router from "./router";

const app = createApp(App);

app.use(createPinia());
app.use(router);
app.use(WinUIOnWeb, { locale: "zh-CN" });

app.mount("#app");
