
<template>
  <v-app>
    <Navbar v-if="path!=`login`"></Navbar>
     <router-view />
    <Footer v-if="path!=`login`"></Footer>
  </v-app>
 
</template>

<script setup lang="ts">

import Footer from './components/Footer.vue';
import Navbar from './components/Navbar.vue';
import { useRouter } from "vue-router";

const path: string | undefined = useRouter().currentRoute?.value.path
console.log(path)
</script>
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
