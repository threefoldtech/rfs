<template>

</template>

<script setup lang="ts">

import { onMounted, ref, watch } from "vue";
import Navbar from "./Navbar.vue";
import Footer from "./Footer.vue";
import image from "../assets/home.png";
import { useClipboard } from "@vueuse/core";
import { toast } from "vue3-toastify";
import "vue3-toastify/dist/index.css";
import { api } from "../client.ts";
import {filesize} from "filesize";


onMounted(async () => {
  try {
    //flists.value = (await api.get<FlistsResponseInterface>("/v1/api/fl")).data;

  } catch (error: any) {
    console.error("Failed to fetch flists", error);
    toast.error(error.response?.data);
  }
});

</script>