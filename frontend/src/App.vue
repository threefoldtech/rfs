<script setup lang="ts">
import { provide, ref } from "vue";
import { LoggedInUser } from "./types/User";

const loggedInUser = ref<string>("");
const updateLoggedInUser = (user: string) => {
  loggedInUser.value = user;
};
provide<LoggedInUser>("loggedInUser", {
  loggedInUser,
  updateLoggedInUser,
});
</script>

<template>
  <router-view />
</template>
<style scoped>
.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: filter 300ms;
}
.logo:hover {
  filter: drop-shadow(0 0 2em #646cffaa);
}
.logo.vue:hover {
  filter: drop-shadow(0 0 2em #42b883aa);
}
</style>
