<template>
  <v-app-bar class="bg-purple-darken-1">
    <v-app-bar-nav-icon class="ml-8">
      <v-img :src="whiteLogo" contain height="50px" width="50px"></v-img>
    </v-app-bar-nav-icon>
    <v-spacer> </v-spacer>
    <v-btn to="Create">Create flist</v-btn>
    <v-btn to="Flists">View List</v-btn>
  </v-app-bar>
</template>

<script setup lang="ts">
import whiteLogo from "../assets/logo_white.png";
</script>
