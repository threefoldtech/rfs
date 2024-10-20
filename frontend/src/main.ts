import { createApp } from "vue";
import "vuetify/styles";
import { createVuetify } from "vuetify";
import * as components from "vuetify/components";
import * as directives from "vuetify/directives";
import App from "./App.vue";
import router from "./router/index";
import createToast from "vue3-toastify";

const toast = createToast;

const vuetify = createVuetify({
  components,
  directives,
});

createApp(App).use(router).use(toast).use(vuetify).mount("#app");
