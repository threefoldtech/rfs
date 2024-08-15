<template>
  <v-app>
    <Navbar />
    <v-main>
      <div>
        <h2 class="ml-5 mt-5">
          <v-icon icon="mdi-account" color="#1aa18f"></v-icon
          >{{ loggedInUser }}
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
            <v-btn class="elevation-0">
              <a :href="baseURL + `/` + value" download>
                <v-icon icon="mdi-download" color="grey"></v-icon
              ></a>
              <v-tooltip activator="parent" location="start"
                >Download flist</v-tooltip
              >
            </v-btn>
            <v-btn @click="copyLink(baseURL + `/` + value)" class="elevation-0">
              <v-icon icon="mdi-content-copy" color="grey"></v-icon>
              <v-tooltip activator="parent">Copy Link</v-tooltip>
            </v-btn>
          </template>
          <template v-else>
            <span>loading... </span>
          </template>
        </template>

        <template #item.last_modified="{ value }">
          {{ new Date(value * 1000).toString() }}
        </template>

        <template v-slot:item.progress="{ value }">
          <template v-if="value != 100">
            <v-progress-linear :model-value="value" color="purple-darken-1">
            </v-progress-linear>
            <span> {{ Math.floor(value) }}% </span>
          </template>
          <template v-else>
            <v-chip color="green">finished</v-chip>
          </template>
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
import { useClipboard } from "@vueuse/core";
import { toast } from "vue3-toastify";
import axios from "axios";

const tableHeader = [
  { title: "Name", key: "name" },
  { title: "Last Modified", key: "last_modified" },
  { title: "Download Link", key: "path_uri", sortable: false },
  { title: "Progress", key: "progress" },
];
const loggedInUser = sessionStorage.getItem("username");
var flists = ref<FlistsResponseInterface>({});
const baseURL = import.meta.env.VITE_API_URL
const api = axios.create({
  baseURL: baseURL,
  headers: {
    "Content-Type": "application/json",
  },
});
let currentUserFlists = computed(() => {
  return loggedInUser?.length ? flists.value[loggedInUser] : [];
});
const { copy } = useClipboard();

const copyLink = (url: string) => {
  copy(url);
  toast.success("Link Copied to Clipboard");
};

onMounted(async () => {
  try {
    flists.value = (await api.get<FlistsResponseInterface>("/v1/api/fl")).data;
    currentUserFlists = computed(() => {
      return loggedInUser?.length ? flists.value[loggedInUser] : [];
    });
  } catch (error:any) {
    console.error("Failed to fetch flists", error);
    toast.error(error.response?.data)
  }
});
</script>
