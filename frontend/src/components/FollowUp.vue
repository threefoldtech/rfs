<template>
  <v-app>
    <Navbar></Navbar>
    <v-main class="d-flex align-center justify-center mb-12 mt-12" height="80%">
      <div v-if="pending" class="text-center">
        <v-progress-circular
          :size="70"
          :width="7"
          color="#1aa18f"
          indeterminate
          class="mb-5"
        >
          <template v-slot:default> {{ progress }} % </template>
        </v-progress-circular>
        <h2 class="mt-12 mb-5">Creating image ...</h2>
        <p>Please wait, your image will be ready in a few minutes.</p>
      </div>
      <div v-else>
        <v-alert
          title="Error Creating Flist"
          type="error"
          v-if="errMsg.length != 0"
          >{{ errMsg }}</v-alert
        >
        <v-alert
          v-else
          title="Image created Succesfully"
          type="success"
        ></v-alert>
      </div>
    </v-main>
    <Footer></Footer>
  </v-app>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import Navbar from "./Navbar.vue";
import Footer from "./Footer.vue";
import { toast } from "vue3-toastify";
import "vue3-toastify/dist/index.css";
import { api } from "../client";

const pending = ref<boolean>(true);
let progress = ref<number>(0);
const errMsg = ref("");
const stopPolling = ref<boolean>(false);
let polling: NodeJS.Timeout;
const uslPartition = window.location.href.split('/')
const id = uslPartition[uslPartition.length - 1]
const pullLists = async () => {
  try {
    const response = await api.get("v1/api/fl/" + id);
    if (response.data.flist_state.InProgress) {
      progress.value = Math.floor(
        response.data.flist_state.InProgress.progress
      );
    } else {
      stopPolling.value = true;
      pending.value = false;
      window.location.href = "/flists" 
    }
  } catch (error: any) {
    console.error("failed to fetch flist status", error);
    pending.value = false;
    errMsg.value = error.response?.data;
    stopPolling.value = true;
    toast.error(error.response?.data)
  }
};

watch(stopPolling, () => {
  if (stopPolling.value) {
    clearInterval(polling);
  }
});

onMounted(() => {
  polling = setInterval(pullLists, 1 * 10000);
});
</script>

<style lang="css" scoped>
.v-progress-circular--indeterminate .v-progress-circular__circle {
  animation: progress-circular-rotate 4s linear infinite !important;
}

@keyframes progress-circular-rotate {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
