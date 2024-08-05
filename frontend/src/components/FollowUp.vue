<template>
  <v-app>
    <Navbar></Navbar>
    <v-main class="d-flex align-center justify-center mb-12 mt-12" height="80%">
      <div v-if="pending" class="text-center">
        <v-progress-circular
          :size="70"
          :width="7"
          color="purple-darken-1"
          indeterminate
          class="mb-5"
        ></v-progress-circular>
        <h2 class="mt-12 mb-5">Creating image . . .</h2>
        <p>Please wait your image will be ready in few minutes.</p>
      </div>
    </v-main>
    <Footer></Footer>
  </v-app>
</template>

<script setup lang="ts">
import { ref } from "vue";
import Navbar from "./Navbar.vue";
import Footer from "./Footer.vue";
const pending = ref<boolean>(true);
</script>
