<template>
   <Navbar/>
    <v-main>

    </v-main>
      

</template>

<script setup lang="ts">
import Navbar from "./Navbar.vue";
import { ref } from "vue";
import { useRouter } from "vue-router";
import { Flist } from "../types/Flist";
import axios from "axios";

const flist = ref<Flist>({ 
    auth: "",
    email: "",
    image_name: "",
    password: "",
    registry_token: "",
    server_address: "",
    username: "", 
});
const router = useRouter();
const api = axios.create({
  baseURL: "http://localhost:4000",
  headers: {
    "Content-Type": "application/json",
  },
});

const create = async () => {
  try {
    await api.post("/v1/api/fl", flist);
  } catch (error) {
    console.error("Failed to create flist", error);
  }
};
</script>

<style>

</style>