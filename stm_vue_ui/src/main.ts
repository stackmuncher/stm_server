import { createApp } from "vue";
import App from "./App.vue";
import router from "@/router";
import VueAuth0Plugin from "vue-auth0-plugin";

createApp(App)
  .use(router)
  .use(VueAuth0Plugin, {
    domain: "stackmuncher.us.auth0.com",
    client_id: "Zf2S4CkHRe9M7l74J1AjDgaxYuooujH0",
    redirect_uri: "http://localhost:3000/",
    scope: "email profile id_token",
    // ... other optional options ...
  })
  .mount("#app");
