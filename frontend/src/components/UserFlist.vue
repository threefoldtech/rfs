<template>
  <v-app>
    <Navbar />
    <v-main>
      <div>
        <h2 class="ml-5 mt-5">
          <v-icon icon="mdi-account" color="purple-darken-1"></v-icon>{{ loggedInUser }}
        </h2>
      </div>
      <v-data-table
        v-if="loggedInUser"
        :items="currentUserFlists"
        :headers="tableHeader"
        hover
      >
        <template v-slot:item.path_uri="{ index, value }">
          <template v-if="currentUserFlists[index].progress === 100">
            <a :href="value" download> Download</a>
          </template>
          <template v-else>
            <span>loading... </span>
          </template>
        </template>

        <template #item.last_modified="{ value }">
          {{ new Date(value * 1000).toString() }}
        </template>
        <template #item.progress="{ value }">
          <v-progress-linear
            :model-value="value"
            color="purple-darken-1"
            
          >
          </v-progress-linear>
          <span> {{ Math.floor(value) }}% </span>
        </template>
      </v-data-table>
    </v-main>
    <Footer />
  </v-app>
</template>
<script setup lang="ts">
import Navbar from "./Navbar.vue";
import Footer from "./Footer.vue";
import { FlistsResponseInterface } from "../types/Flists.ts";
import { computed } from "vue";
import { onMounted, ref } from "vue";
import axios from "axios";

const tableHeader = [
  { title: "Name", key: "name" },
  { title: "Is File", key: "is_file" },
  { title: "Last Modified", key: "last_modified" },
  { title: "Download Link", key: "path_uri", sortable: false },
  { title: "Progress", key: "progress" },
];
const loggedInUser = sessionStorage.getItem("username");
var flists = ref<FlistsResponseInterface>({});
const api = axios.create({
  baseURL: "http://localhost:4000",
  headers: {
    "Content-Type": "application/json",
  },
});
let currentUserFlists = computed(() => {
  return loggedInUser?.length ? flists.value[loggedInUser] : [];
});
onMounted(async () => {
  try {
    flists.value = (await api.get<FlistsResponseInterface>("/v1/api/fl")).data;
    currentUserFlists = computed(() => {
      return loggedInUser?.length ? flists.value[loggedInUser] : [];
    });
  } catch (error) {
    console.error("Failed to fetch flists", error);
  }
});
</script>
