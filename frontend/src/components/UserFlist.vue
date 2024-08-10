<template>
  <v-app>
    <Navbar />
    <v-main>
      <v-data-table
        v-if="loggedInUser"
        :items="currentUserFlists"
        :headers="tableHeader"
        hover
      >
        <template #item.last_modified="{ value }">
          {{ new Date(value * 1000).toString() }}
        </template>
        <template #item.progress="{ value }">
          <v-progress-linear
            :model-value="value"
            color="purple-darken-1"
            class="w-75"
          >
          </v-progress-linear>
          <span> {{ value }}% </span>
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
  return loggedInUser?.length
    ? flists.value[loggedInUser]
    : [];
});
onMounted(async () => {
  try {
    flists.value = (await api.get<FlistsResponseInterface>("/v1/api/fl")).data;
      currentUserFlists = computed(() => {
        return loggedInUser?.length
          ? flists.value[loggedInUser]
          : [];
      });
  } catch (error) {
    console.error("Failed to fetch flists", error);
  }
});


</script>
