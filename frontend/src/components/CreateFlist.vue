<template>
  <v-app>
    <Navbar />
    <v-main class="d-flex flex-column justify-center" height="90%">
      <v-container fluid>
        <v-row justify="center">
          <v-col :cols="8">
            <h2 class="mb-2">Create a Flist:</h2>
          </v-col>
        </v-row>
        <v-row justify="center">
          <v-col :cols="4">
            <label
              for="image-name"
              class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
            >
              Image Name
            </label>
            <v-text-field
              class="pr-5 rounded"
              id="image-name"
              v-model="flist.image_name"
              variant="solo-filled"
              required
            >
            </v-text-field>
            <label
              for="email"
              class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
            >
              Email
            </label>
            <v-text-field
              class="pr-5 rounded"
              id="email"
              v-model="flist.email"
              variant="solo-filled"
            >
            </v-text-field>
            <label
              for="auth"
              class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
            >
              Auth
            </label>
            <v-text-field
              class="pr-5 rounded"
              id="auth"
              v-model="flist.auth"
              variant="solo-filled"
            >
            </v-text-field>
            <label
              for="registery-token"
              class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
            >
              Registery Token
            </label>
            <v-text-field
              class="pr-5 rounded mb-5"
              id="registery-token"
              v-model="flist.registry_token"
              variant="solo-filled"
            >
            </v-text-field>

            <v-btn
              class="pr-5 rounded-pill bg-purple-darken-1 mb-8"
              size="large"
              width="50%"
              @click="create"
              >Create</v-btn
            >
          </v-col>
          <v-col :cols="4">
            <label
              for="username"
              class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
            >
              Username
            </label>
            <v-text-field
              class="pr-5 rounded"
              id="username"
              v-model="flist.username"
              variant="solo-filled"
            >
            </v-text-field>

            <label
              for="password"
              class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
            >
              Password
            </label>
            <v-text-field
              class="pr-5 rounded"
              id="password"
              v-model="flist.password"
              variant="solo-filled"
              :append-inner-icon="visible ? 'mdi-eye-off' : 'mdi-eye'"
              :type="visible ? 'text' : 'password'"
              @click:append-inner="visible = !visible"
            >
            </v-text-field>
            <label
              for="server-address"
              class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
            >
              Server Address
            </label>
            <v-text-field
              class="pr-5 rounded"
              id="server-address"
              v-model="flist.server_address"
              variant="solo-filled"
            >
            </v-text-field>
          </v-col>
        </v-row>
      </v-container>
    </v-main>
    <Footer></Footer>
  </v-app>
</template>

<script setup lang="ts">
import Navbar from "./Navbar.vue";
import { ref } from "vue";
import { useRouter } from "vue-router";
import { Flist } from "../types/Flist";
import axios from "axios";
import Footer from "./Footer.vue";

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
const visible = ref<boolean>(false);
const create = async () => {
  try {
    await api.post("/v1/api/fl", flist.value);
    router.push("Follow");
  } catch (error) {
    console.error("Failed to create flist", error);
  }
};
</script>

<style scoped></style>
