<template>
  <v-app>
    <Navbar></Navbar>
    <div class="w-100 position-relative" style="height: 10%">
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
        style="top: 10%; left: 0; height: fit-content"
        v-model="drawer"
        :rail="rail"
        @click="rail = !rail"
      >
        <v-list>
          <v-list-item title="Users" nav>
            <template v-slot:append>
              <v-btn variant="text" @click.stop="rail = !rail">
                <v-icon>{{
                  !rail ? "mdi-chevron-left" : "mdi-chevron-right"
                }}</v-icon></v-btn
              >
            </template>
          </v-list-item>
          <v-divider v-if="!rail"</v-divider>
          <v-list-item
            v-for="[key, _] in flists.flists"
            title="key"
            :key="key"
            @click="username = key"
          ></v-list-item>
        </v-list>
      </v-navigation-drawer>
      <v-container
        class="elevation-2 d-flex w-75"
        fluid
        style="height: fit-content"
      >
        <!-- table containe flists -->
        <v-data-table :items="filteredFlist" :headers="tableHeader" hover>
        </v-data-table>
        <!-- <v-table class="elevation-1">
              <thead>
                <tr>
                  <th class="text-left">Name</th>
                  <th class="text-left">Last Modified</th>
                  <th class="text-left">Path URI</th>
                </tr>
              </thead>
              <tbody >
                <tr v-for="item in filteredFlist">
                  <td>{{ item.name }}</td>
                  <td>{{ item.lastModified }}</td>
                  <td>{{ item.pathUri }}</td>
                </tr>
              </tbody>
            </v-table> -->
      </v-container>
    </v-main>
    <Footer></Footer>
  </v-app>
</template>

<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import axios from "axios";
import Navbar from "./Navbar.vue";
import Footer from "./Footer.vue";
import image from "../assets/side.png";
import { FlistsResponseInterface, FlistBody } from "../types/Flists.ts";

const api = axios.create({
  baseURL: "http://localhost:4000",
  headers: {
    "Content-Type": "application/json",
  },
});
const rail = ref<boolean>(true);
const drawer = ref<boolean>(true);

const tableHeader = [
  { title: "Name", key: "name" },
  { title: "Last Modified", key: "  lastModified" },
  { title: "Download Link", key: "pathUri", sortable: false },
];
const flists = ref<FlistsResponseInterface>({
  flists: new Map<string, FlistBody[]>(),
});
const username = ref("");
let filteredFlist = ref<FlistBody[]>([]);
const filteredFlistFn = () => {
  filteredFlist.value = [];
  if (username.value.length === 0) {
    flists.value.flists.forEach((flistMap, _) => {
      for (let flist of flistMap) {
        if (flist.progress === 100) {
          filteredFlist.value.push(flist);
        }
      }
    });
  } else {
    flists.value.flists.get(username.value)?.forEach((flist) => {
      if (flist.progress === 100) {
        filteredFlist.value.push(flist);
      }
    });
  }
};
onMounted(async () => {
  try {
    flists.value = await api.get("/v1/api/fl");
    filteredFlistFn();
  } catch (error) {
    console.error("Failed to fetch flists", error);
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
