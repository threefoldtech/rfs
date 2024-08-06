<template>
  <v-app>
    <Navbar></Navbar>
    <v-main class="d-flex justify-center">
        <v-navigation-drawer
          v-model="drawer"
          :rail="rail"
          @click="rail = !rail"
          elevation="2"
          width="20%"
          app
        >
          <v-list>
            <v-list-item title="Users" nav></v-list-item>
          </v-list>
        </v-navigation-drawer>
        <v-btn icon @click.stop="drawer = !drawer" class="rounded-1">
          <v-icon>{{
            drawer ? "mdi-chevron-left" : "mdi-chevron-right"
          }}</v-icon>
        </v-btn>
      <v-container class="elevation-2 mt-5 mb-5">
        <v-row>
          <v-col cols="3"> </v-col>
          <v-col cols="9"> </v-col>
        </v-row>
      </v-container>
    </v-main>
    <Footer></Footer>
  </v-app>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import axios from "axios";
import Navbar from "./Navbar.vue";
import Footer from "./Footer.vue";

const drawer = ref<boolean>(true);
const rail = ref<boolean>(true);

const api = axios.create({
  baseURL: "http://localhost:4000",
  headers: {
    "Content-Type": "application/json",
  },
});
const flists: any = ref("");
onMounted(async () => {
  try {
    flists.value = await api.get("/v1/api/fl");
  } catch (error) {
    console.error("Failed to fetch flists", error);
  }
});
</script>

<style></style>
