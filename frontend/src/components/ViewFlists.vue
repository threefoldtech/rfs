<template>
    <div>hello</div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import axios from "axios";

const api = axios.create({
  baseURL: "http://localhost:4000",
  headers: {
    "Content-Type": "application/json",
  },
});
const flists: any = ref('')
onMounted(async () =>{
    try{
        flists.value = await api.get("/v1/api/fl")
    } catch (error) {
        console.error("Failed to fetch flists", error);
    }
})
</script>

<style>

</style>