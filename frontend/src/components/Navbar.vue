<template>
  <v-app-bar color="#1aa18f">
    <v-app-bar-nav-icon to="/" class="ml-8">
      <v-img :src="whiteLogo" contain height="50px" width="50px"></v-img>
    </v-app-bar-nav-icon>
    <v-spacer> </v-spacer>
    <div class="mr-5" v-if="auth === null || auth?.length === 0">
      <v-btn to="Login">Login</v-btn>
    </div>
    <div class="mr-5" v-else>
      <v-btn to="/Create"
        ><v-icon icon="mdi-plus-circle-outline" class="mr-2"></v-icon>Create
        flist</v-btn
      >
      <v-menu class="white">
        <template v-slot:activator="{ props }">
          <v-btn
            class="align-self-center me-4"
            height="100%"
            rounded="50%"
            variant="plain"
            v-bind="props"
            style="font-size: 20px"
          >
            <v-icon icon="mdi-account"></v-icon>
          </v-btn>
        </template>
        <v-list>
          <v-list-item>
            <v-btn><a href="/flists" class="text-black" style="text-decoration:none;">My FLists</a></v-btn>
          </v-list-item>
          <v-list-item>
            <v-btn @click="logout"
              ><v-icon icon="mdi-logout" style="font-size: 20px" />log
              out</v-btn
            >
          </v-list-item>
        </v-list>
      </v-menu>
    </div>
  </v-app-bar>
</template>

<script setup lang="ts">
import whiteLogo from "../assets/logo_white.png";
import { toast } from "vue3-toastify";
const auth: string | null = sessionStorage.getItem("token");

const logout = async () => {
  try {
    sessionStorage.removeItem("token")
    sessionStorage.removeItem("username")
    window.location.href = "/"
  } catch (error: any) {
    console.error("Failed to logout", error);
    toast.error(error.response?.data || "error occured");
  }
};

</script>
