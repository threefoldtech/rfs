<template>
  <v-container
    fluid
    style="
      padding: 0px;
      max-height: 100vh;
      overflow-y: hidden;
      overflow-x: hidden;
    "
  >
    <v-row>
      <v-col :cols="4">
        <v-img :src="image" cover height="100%"> </v-img>
      </v-col>
      <v-col :cols="8" class="d-flex align-center">
        <v-container class="d-flex flex-column align-center justify-center">
          <v-col :cols="6">
            <v-form>
              <v-img :src="logo" class="mb-10" height="10%" width="15%"></v-img>
              <h2 class="mb-5">Sign in</h2>

              <label
                for="username"
                class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
              >
                Username
              </label>
              <v-text-field
                class="pr-5 rounded"
                v-model="user.username"
                variant="outlined"
                density="compact"
                id="username"
                required
              >
              </v-text-field>
              <label
                for="password"
                class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
              >
                Password
              </label>
              <v-text-field
                class="mb-5 pr-5 rounded"
                v-model="user.password"
                :append-inner-icon="visible ? 'mdi-eye-off' : 'mdi-eye'"
                :type="visible ? 'text' : 'password'"
                variant="outlined"
                @click:append-inner="visible = !visible"
                density="compact"
                id="password"
                required
              >
              </v-text-field>
              <v-btn
                class="pr-5 rounded-pill bg-purple-darken-1"
                size="large"
                width="50%"
                @click="login"
                >Sign In</v-btn
              >
            </v-form>
          </v-col>
        </v-container>
      </v-col>
    </v-row>
  </v-container>
</template>

<script setup lang="ts">
import { ref } from "vue";
import image from "./../assets/Image.png";
import logo from "./../assets/logo.png";
import { useRouter } from "vue-router";
import { User } from "../types/User.ts";
import axios from "axios";

const user = ref<User>({ username: "", password: "" });
const router = useRouter();
const api = axios.create({
  baseURL: "http://localhost:4000",
  headers: {
    "Content-Type": "application/json",
  },
});
const visible = ref<boolean>(false);

const login = async () => {
  try {
    const response = await api.post("/v1/api/signin", user.value);
    const token = response.data.access_token;
    sessionStorage.setItem("token", token);
    router.push("/flists");
  } catch (error) {
    console.error("Failed to login", error);
  }
};
</script>
