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
            v-for="userName in userNameList"
            title="userName"
            :key="userName"
            @click="username = userName"
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
  { title: "Last Modified", key: "last_modified" },
  { title: "Download Link", key: "path_uri", sortable: false },
];
var flists = ref<FlistsResponseInterface>({})
const username = ref("");
const userNameList = ref<string[]>([])
let filteredFlist = ref<FlistBody[]>([]);
const filteredFlistFn = () => {
  filteredFlist.value = [];
  const map = flists.value;
  if (username.value.length === 0) {
    for(var flistMap in map){
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
const getUserNames = () =>{
  filteredFlist.value = [];
  userNameList.value = [];
  const map = flists.value;
  for(var flistMap in map){
    userNameList.value.push(flistMap)
  }
}
onMounted(async () => {
  try {
    flists.value = (await api.get<FlistsResponseInterface>("/v1/api/fl")).data;
    getUserNames();
    console.log(userNameList.value)
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
