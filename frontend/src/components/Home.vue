<template>
  <v-app>
    <Navbar></Navbar>
    <div class="w-100 position-relative" style="height: 30%">
      <v-img :src="image" cover style="z-index: 2"></v-img>
      <div
        class="position-absolute text-white"
        style="z-index: 4; top: 40%; left: 35%"
      >
        <h1>Create and Download Flist</h1>
      </div>
    </div>

    <v-main class="d-flex justify-center mt-0" style="height: fit-content">
      <v-navigation-drawer
        elevation="2"
        app
        class="position-absolute mx-height"
        style="top: 30%; left: 0; height: fit-content"
      >
        <v-list>
          <v-list-item nav>
            <v-list-item-title class="text-h6"> Users</v-list-item-title>
            <template v-slot:append>
              <v-btn variant="text" @click.stop="collapsed = !collapsed">
                <v-icon>{{
                  !collapsed ? "mdi-chevron-up" : "mdi-chevron-down"
                }}</v-icon></v-btn
              >
            </template>
          </v-list-item>
          <v-divider v-if="!collapsed"></v-divider>
          <v-list-item
            v-if="!collapsed"
            v-for="userName in userNameList"
            :key="userName"
            @click="username = userName"
          >
            <template v-slot:prepend>
              <v-icon icon="mdi-account" color="#1aa18f"></v-icon>
            </template>
            <v-list-item-title>
              {{ userName }}
            </v-list-item-title>
          </v-list-item>
        </v-list>
      </v-navigation-drawer>
      <v-container
        class="d-flex flex-column w-75"
        fluid
        style="height: fit-content"
      >
        <h2 class="mb-2" v-if="username.length != 0">
          <v-icon icon="mdi-account" color="#1aa18f"></v-icon>{{ username }}
        </h2>
        <!-- table containe flists -->
        <v-data-table
          :items="filteredFlist"
          :headers="tableHeader"
          hover
          class="elevation-2"
        >
          <template #item.last_modified="{ value }">
            {{ new Date(value * 1000).toString() }}
          </template>
          <template #item.path_uri="{ value }">
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
        </v-data-table>
      </v-container>
    </v-main>
    <Footer></Footer>
  </v-app>
</template>

<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import Navbar from "./Navbar.vue";
import Footer from "./Footer.vue";
import image from "../assets/home.png";
import { useClipboard } from "@vueuse/core";
import { FlistsResponseInterface, FlistBody } from "../types/Flists.ts";
import { toast } from "vue3-toastify";
import "vue3-toastify/dist/index.css";
import { api } from "../client.ts";

const baseURL = import.meta.env.VITE_API_URL;
const collapsed = ref<boolean>(true);

const copyLink = (url: string) => {
  copy(url);
  toast.success("Link Copied to Clipboard");
};

const tableHeader = [
  { title: "Name", key: "name" },
  { title: "Last Modified", key: "last_modified" },
  { title: "Download Link", key: "path_uri", sortable: false },
];
var flists = ref<FlistsResponseInterface>({});
const username = ref("");
const userNameList = ref<string[]>([]);
let filteredFlist = ref<FlistBody[]>([]);
const { copy } = useClipboard();
const filteredFlistFn = () => {
  filteredFlist.value = [];
  const map = flists.value;
  if (username.value.length === 0) {
    for (var flistMap in map) {
      for (let flist of map[flistMap]) {
        if (flist.progress === 100) {
          filteredFlist.value.push(flist);
        }
      }
    }
  } else {
    for (let flist of map[username.value]) {
      if (flist.progress === 100) {
        filteredFlist.value.push(flist);
      }
    }
  }
};
const getUserNames = () => {
  const list: string[] = [];
  const map = flists.value;
  for (var flistMap in map) {
    list.push(flistMap)
  }
  userNameList.value=list
};
onMounted(async () => {
  try {
    flists.value = (await api.get<FlistsResponseInterface>("/v1/api/fl")).data;
    getUserNames();
    filteredFlistFn();
  } catch (error: any) {
    console.error("Failed to fetch flists", error);
    toast.error(error.response?.data);
  }
});
watch(username, () => {
  filteredFlistFn();
});
</script>
<style lang="css" scoped>
.mx-height {
  max-height: 600px;
}
</style>
