
<template>
  <v-app>
    <Navbar v-if="path !== `login`"></Navbar>
    <v-main class="mn-height" >
      <router-view v-slot="{ Component, route }">
        <div :key="route.path">
          <component :is="Component" />
        </div>
      </router-view>
    </v-main>
    <Footer v-if="path !== `login`"></Footer>
  </v-app>
</template>

<script setup lang="ts">
import Footer from './components/Footer.vue';
import Navbar from './components/Navbar.vue';
import { useRoute } from 'vue-router';
var path = useRoute().name?.toString()
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
