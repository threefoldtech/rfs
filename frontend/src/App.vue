
<template>
  <v-app>
      <router-view v-slot="{ Component, route }">
        <Navbar v-if="route.path != `/login`"></Navbar>
        <v-main class="mn-height" style="--v-layout-left: 0px;" >
        <div :key="route.path">
          <component :is="Component" />
        </div>
        </v-main>
         <Footer v-if="route.path != `/login`"></Footer>
      </router-view>
  </v-app>
</template>

<script setup lang="ts">
import Footer from './components/Footer.vue';
import Navbar from './components/Navbar.vue';
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
