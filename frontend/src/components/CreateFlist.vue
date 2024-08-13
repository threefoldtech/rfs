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
          <v-col :cols="8">
            <label
              for="image-name"
              class="text-subtitle-1 text-medium-emphasis d-flex align-center"
            >
              Image Name<span style="color: red">*</span>
            </label>

            <v-text-field
              class="pr-5 rounded"
              id="image-name"
              v-model="flist.image_name"
              variant="solo-filled"
              density="compact"
              required
              placeholder="example: redis, keinos/sqlite3, alpine"
            >
            </v-text-field>
            <v-checkbox
              value="true"
              v-model="privateReg"
              hide-details
              density="compact"
              ><template v-slot:label>
                <span class="text-subtitle-2">Private Registery</span>
              </template>
            </v-checkbox>

            <div v-if="privateReg">
              <v-radio-group class="p-0 m-0" v-model="privateType" inline>
                <v-radio value="username">
                  <template v-slot:label>
                    <span class="text-subtitle-2">Username - Password</span>
                  </template>
                </v-radio>
                <v-radio value="email">
                  <template v-slot:label>
                    <span class="text-subtitle-2">Email - Password</span>
                  </template>
                </v-radio>
                <v-radio value="token">
                  <template v-slot:label>
                    <span class="text-subtitle-2">Identity Token</span>
                  </template>
                </v-radio>
              </v-radio-group>
              <v-container class="pr-0 pl-0">
                <v-row>
                  <v-col>
                    <div v-if="privateType === `email`">
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
                        density="compact"
                        placeholder="johndoe@gmail.com"
                        type="email"
                      >
                      </v-text-field>
                    </div>
                    <div v-if="privateType !== `email`">
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
                        density="compact"
                        :placeholder="
                          privateType === `token` ? `token` : `johndoe`
                        "
                      >
                      </v-text-field>
                    </div>
                  </v-col>
                  <v-col>
                    <div
                      v-if="privateType.length != 0 && privateType !== `token`"
                    >
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
                        density="compact"
                      >
                      </v-text-field>
                    </div>
                    <div v-if="privateType === `token`">
                      <label
                        for="identity-token"
                        class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
                      >
                        Identity Token
                      </label>
                      <v-text-field
                        class="pr-5 rounded"
                        id="identity-token"
                        v-model="flist.identity_token"
                        variant="solo-filled"
                        density="compact"
                      >
                      </v-text-field>
                    </div>
                  </v-col>
                </v-row>
              </v-container>
            </div>

            <v-checkbox
              value="true"
              v-model="registeryAddress"
              hide-details
              density="compact"
              ><template v-slot:label>
                <span class="text-subtitle-2">Self Hosted Registery</span>
              </template>
            </v-checkbox>
            <div v-if="registeryAddress">
              <label
                for="server-address"
                class="text-subtitle-1 text-medium-emphasis d-flex align-center justify-space-between"
              >
                Registery Address
              </label>
              <v-text-field
                class="pr-5 rounded"
                id="server-address"
                v-model="flist.server_address"
                variant="solo-filled"
                density="compact"
                placeholder="localhost:5000/ubuntu"
              >
              </v-text-field>
            </div>
            <v-checkbox
              value="true"
              v-model="registeryToken"
              density="compact"
              hide-details
              ><template v-slot:label>
                <span class="text-subtitle-2">Web Registery Token</span>
              </template>
            </v-checkbox>
            <div v-if="registeryToken">
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
                density="compact"
              >
              </v-text-field>
            </div>

            <v-btn
              class="pr-5 rounded-pill bg-purple-darken-1 mb-8"
              size="large"
              width="50%"
              @click="create"
              >Create</v-btn
            >
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
import { toast } from "vue3-toastify";
import "vue3-toastify/dist/index.css";

const privateReg = ref<boolean>(false);
const registeryAddress = ref<boolean>(false);
const registeryToken = ref<boolean>(false);
const privateType = ref<string>("username");

const flist = ref<Flist>({
  auth: "",
  email: "",
  identity_token: "",
  image_name: "",
  password: "",
  registry_token: "",
  server_address: "",
  username: "",
});
const router = useRouter();
const api = axios.create({
  baseURL: import.meta.env.VITE_API_URL,
  headers: {
    "Content-Type": "application/json",
    Authorization: "Bearer " + sessionStorage.getItem("token"),
  },
});
const visible = ref<boolean>(false);
const create = async () => {
  try {
    const response = await api.post("/v1/api/fl", flist.value);
    router.push({ name: "FollowUp", params: { id: response.data.id } });
  } catch (error: any) {
    console.error("Failed to create flist", error);
    toast.error(error.response?.data || "error occured");
    const errors: Number[] = [401, 403];
    if (errors.includes(error.response?.status)) {
      sessionStorage.removeItem("token");
    }
  }
};
</script>
