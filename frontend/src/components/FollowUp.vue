<template>
  <v-app>
    <Navbar></Navbar>
    <v-main class="d-flex align-center justify-center mb-12 mt-12" height="80%">
      <div v-if="pending" class="text-center">
        <v-progress-circular
          :size="70"
          :width="7"
          color="purple-darken-1"
          indeterminate
          class="mb-5"
        >
          <template v-slot:default> {{ progress }} % </template>
        </v-progress-circular>
        <h2 class="mt-12 mb-5">Creating image . . .</h2>
        <p>Please wait your image will be ready in few minutes.</p>
      </div>
      <div v-else>
        <v-alert
          title="Error Creating Flist"
          type="error"
          v-if="errMsg.len != 0"
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
import { useRoute, useRouter } from "vue-router";
import Footer from "./Footer.vue";
import axios from "axios";

const pending = ref<boolean>(true);
const api = axios.create({
  baseURL: "http://localhost:4000",
  headers: {
    "Content-Type": "application/json",
    Authorization: "Bearer " + sessionStorage.getItem("token"),
  },
});
const route = useRoute();
let progress = ref<number>(0);
const router = useRouter();
var id = route.params.id;
const errMsg = ref("");
const stopPolling = ref<boolean>(false);
let polling;
const pullLists = async () => {
  try {
    const response = await api.get("v1/api/fl/" + id);
    if (response.data.flist_state.InProgress) {
      console.log("loading");
      progress.value = Math.floor(
        response.data.flist_state.InProgress.progress
      );
    } else {
      console.log("done");
      stopPolling.value = true;
      pending.value = false;
      router.push("/flists");
    }
  } catch (error: any) {
    console.error("failed to fetch flist status", error);
    pending.value = false;
    errMsg.value = error.response?.data;
    stopPolling.value = true;
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
