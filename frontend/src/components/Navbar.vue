<template>
  <v-app-bar color="#1aa18f" height="125px">
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
      <v-btn to="/Flists">My FLists</v-btn>
    </div>
  </v-app-bar>
</template>

<script setup lang="ts">
import whiteLogo from "../assets/logo_white.png";
import "../../public/theme.css";
const auth: string | null = sessionStorage.getItem("token");
</script>
