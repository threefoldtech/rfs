<template>
  <v-app-bar class="bg-purple-darken-1">
    <v-app-bar-nav-icon to="Home" class="ml-8">
      <v-img :src="whiteLogo" contain height="50px" width="50px"></v-img>
    </v-app-bar-nav-icon>
    <v-spacer> </v-spacer>
    <div class="mr-5" v-if="auth === null || auth?.length === 0">
      <v-btn to="Login">Login</v-btn>
    </div>
    <div class="mr-5" v-else>
      <v-btn to="Create">Create flist</v-btn>
      <v-btn to="Flists">View List</v-btn>
    </div>
  </v-app-bar>
</template>

<script setup lang="ts">
import whiteLogo from "../assets/logo_white.png";
const auth: string | null = sessionStorage.getItem("token");
</script>
