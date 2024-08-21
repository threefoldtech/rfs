<template>
  <v-app>
    <Navbar></Navbar>
    <div class="w-100 position-relative">
      <v-img :src="image" cover style="z-index: 2"></v-img>
      <div
        class="position-absolute w-100 text-white d-flex justify-content align-content "
        style="z-index: 4; top: 55%;left:40%;"
      >
        <h1 class="text-h4">{{id}}</h1>
      </div>
    </div>
    <v-main class="mn-height">
      <v-container class="m-0 pa-0">
        <v-row>
          <div>
            <h2 class="text-h4">{{
              id
              }}</h2>
            <p>This Flist was created by <v-chip color="#1aa18f" label>{{ username }} </v-chip> </p>
          </div>
        </v-row>
        <v-row class="d-flex flex-column">
            <h3 class="text-h5">Source file</h3>
            <v-text-field rounded="20" variant="outlined" density="compact" readonly class="text-grey-darken-1 mr-0">
              {{ url }}
              <template #append>
        <v-btn
            color="#1aa18f"
            value="Copy"
            class="Btn"
            @click="copyLink(url)">
        Copy
        </v-btn>
    </template>
            </v-text-field>
        </v-row>
          <v-row class="d-flex flex-column">
            <h3 class="text-h5">Content</h3>
            <v-textarea :model-value="showContent" variant="outlined" readonly class="text-grey-darken-1" auto-grow width="98.5%">
            </v-textarea>
        </v-row>
      </v-container>
    </v-main>
    <Footer />
  </v-app>
</template>

<script setup lang="ts">

import { onMounted, ref } from "vue";
import Navbar from "./Navbar.vue";
import Footer from "./Footer.vue";
import image from "../assets/home.png";
import { toast } from "vue3-toastify";
import "vue3-toastify/dist/index.css";
import { api } from "../client.ts";
import { copyLink } from "../client.ts";


const content = ref<string[]>([]);
  const urlPartition = window.location.href.split("/")
  const id = ref<string>(urlPartition[urlPartition.length - 1])
  const username = ref<string>(urlPartition[urlPartition.length - 2])
  const baseURL = import.meta.env.VITE_API_URL;
const url = baseURL + "/flists" + "/" + username.value + "/" + id.value

const showContent = ref<string>()
onMounted(async () => {
  try {
    content.value = (await api.get(url)).data;
    content.value = content.value.slice(1)
    showContent.value = content.value.join("\n")

  } catch (error: any) {
    console.error("Failed to fetch flists", error);
    toast.error(error.response?.data);
  }
});

</script>

<style scoped>
.Btn{
  position: relative;
  left: -18px;
  height: 50px;
  width: 110px;
  margin-left:0px;
}
</style>