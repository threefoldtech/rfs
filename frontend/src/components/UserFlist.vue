<template>
    <div class="mn-height">
      <v-container class="pa-0">
        <v-row no-gutters class="pa-0 ma-0">
          <div class="user">
            <h2 class="mt-5 mb-5 text-h5 text-grey-darken-2">
              <v-icon icon="mdi-account" color="#1aa18f"></v-icon
              >{{ loggedInUser }}
            </h2>
          </div>
        </v-row>
        <v-row no-gutters class="pa-0 ma-0">
          <v-data-table
            density="compact"
            v-if="loggedInUser"
            :items="currentUserFlists"
            :headers="tableHeader"
            dense
            items-per-page="25"
            class="thick-border"
          >
            <template #item.name="{ value }">
              <v-icon icon="mdi-text-box" class="mr-1" color="grey" />
              <span class="file-name">{{ value }}</span>
            </template>
              <template v-slot:item.preview = "{index}" >
          <a :href="`/` + currentUserFlists[index].path_uri">
            <v-btn class="elevation-0">
                  <v-icon icon="mdi-eye-outline" color="grey"></v-icon>
            </v-btn>
          </a>
        </template>
            <template #item.size="{ value }">
              {{ filesize(value, { standard: "jedec", precision: 3 }) }}
            </template>
            <template v-slot:item.path_uri="{ index, value }">
              <template v-if="currentUserFlists[index].progress === 100">
                <v-btn class="elevation-0">
                  <a :href="baseURL + `/` + value" download>
                    <v-icon icon="mdi-download" color="grey"></v-icon
                  ></a>
                  <v-tooltip activator="parent" location="start"
                    >Download flist</v-tooltip
                  >
                </v-btn>
                <v-btn
                  @click="copyLink(baseURL + `/` + value)"
                  class="elevation-0"
                >
                  <v-icon icon="mdi-content-copy" color="grey"></v-icon>
                  <v-tooltip activator="parent">Copy Link</v-tooltip>
                </v-btn>
              </template>
              <template v-else>
                <span>loading... </span>
              </template>
            </template>

            <template #item.last_modified="{ value }">
              {{ new Date(value * 1000).toString().split("(")[0] }}
            </template>

            <template v-slot:item.progress="{ value }" class="w-25">
              <template v-if="value != 100">
                <v-progress-linear
                  :model-value="value"
                  color="#1aa18f"
                  height="20"
                  rounded="sm"
                >
                  <template v-slot:default="{ value }">
                    <span class="text-white">{{ Math.floor(value) }}%</span>
                  </template>
                </v-progress-linear>
              </template>
              <template v-else>
                <v-chip color="#1aa18f">finished</v-chip>
              </template>
            </template>
          </v-data-table>
        </v-row>
      </v-container>
    </div>
</template>
<script setup lang="ts">
import { FlistsResponseInterface } from "../types/Flist.ts";
import { computed } from "vue";
import { onMounted, ref } from "vue";
import { toast } from "vue3-toastify";
import { api } from "../client.ts";
import { copyLink } from "../helpers.ts";
import { filesize } from "filesize";




const tableHeader = [
  { title: "File Name", key: "name" },
  { title: "Preview", key:"preview"},
  { title: "Size", key: "size" },
  { title: "Last Modified", key: "last_modified" },
  { title: "Download", key: "path_uri", sortable: false },
  { title: "Progress", key: "progress", width: "20%" },
];
const loggedInUser = sessionStorage.getItem("username");
var flists = ref<FlistsResponseInterface>({});
const baseURL = import.meta.env.VITE_API_URL;
let currentUserFlists = computed(() => {
  return loggedInUser?.length ? flists.value[loggedInUser] : [];
});

onMounted(async () => {
  try {
    flists.value = (await api.get<FlistsResponseInterface>("/v1/api/fl")).data;
    currentUserFlists = computed(() => {
      return loggedInUser?.length ? flists.value[loggedInUser] : [];
    });
  } catch (error: any) {
    console.error("Failed to fetch flists", error);
    toast.error(error.response?.data);
  }
});
</script>

<style scoped>
.user .v-icon--size-default {
  font-size: 25px;
}

</style>
