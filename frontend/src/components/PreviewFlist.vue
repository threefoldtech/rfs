<template>
    <div class="w-100 position-relative" style="top: -62.5px">
      <v-img :src="image" cover style="z-index: 2"></v-img>
      <div
        class="position-absolute w-100 text-white d-flex justify-content align-content "
        style="z-index: 4; top: 55%;left:40%;"
      >
      </div>
    </div>
    <div class="mn-height mb-10" v-if="!pending">
      <v-container class="m-0 pa-0">
        <v-row>
          <div>
            <h2 class="text-h4 mb-3">{{
              id
              }}</h2>
            <p>This Flist was created by <v-chip color="#1aa18f" label>{{ username }} </v-chip> </p>
          </div>
        </v-row>
        <v-row class="d-flex flex-column">
            <h3 class="text-subtitle-1 text-grey-darken-2">Source file</h3>
            <v-text-field rounded="20" variant="outlined" density="compact" readonly class="text-grey-darken-1 mr-0">
              {{ baseURL + url }}
              <template #append>
                <v-btn
                    color="#1aa18f"
                    value="Copy"
                    class="Btn"
                    @click="copyLink(baseURL + url)">
                Copy
                </v-btn>
              </template>
            </v-text-field>
        </v-row>
        <v-row class="d-flex flex-column">
            <h3 class="text-subtitle-1 text-grey-darken-2">Archive Checksum (MD5)</h3>
            <v-text-field rounded="20" variant="outlined" density="compact" readonly class="text-grey-darken-1 mr-0">
              {{flistPreview.checksum}}
              <template #append>
                <v-btn
                    color="#1aa18f"
                    value="Copy"
                    class="Btn"
                    @click="copyLink(flistPreview.checksum)">
                Copy
                </v-btn>
              </template>
            </v-text-field>
        </v-row>
        <v-row class="d-flex flex-column">
            <h3 class="text-subtitle-1 text-grey-darken-2">Metadata</h3>
            <v-text-field rounded="20" variant="outlined" density="compact" readonly class="text-grey-darken-1 mr-0" width="98.5%">
              {{ flistPreview.metadata}}
              <template #prepend-inner>
       <v-chip color="#1aa18f" label class ="chip">Backend (default)</v-chip>
    </template>
            </v-text-field>
        </v-row>
          <v-row class="d-flex flex-column">
            <h3 class="text-subtitle-1 text-grey-darken-2">Content</h3>
            <v-textarea :model-value="showContent" variant="outlined" readonly rows="1" :class= "linkDecoration" class="text-grey-darken-1"  auto-grow width="98.5%" @click="contentShow()">
            </v-textarea>
        </v-row>
      </v-container>
    </div>
    <div class="d-flex align-center justify-center mb-12 mt-12"  v-else>
       <v-progress-circular
          :size="70"
          :width="7"
          color="#1aa18f"
          indeterminate
          class="mb-5"
        >
        </v-progress-circular>
    </div>
</template>

<script setup lang="ts">

import { onMounted, ref } from "vue";
import image from "../assets/home.png";
import { toast } from "vue3-toastify";
import "vue3-toastify/dist/index.css";
import { api } from "../client.ts";
import { copyLink } from "../helpers.ts";
import { FlistPreview  } from "../types/Flist.ts";

const pending = ref<boolean>(true)
const flistPreview = ref<FlistPreview>({checksum:"", content:[], metadata:""});
const urlPartition = window.location.href.split("/")
const id = ref<string>(urlPartition[urlPartition.length - 1])
const username = ref<string>(urlPartition[urlPartition.length - 2])
const baseURL = ref<string>(import.meta.env.VITE_API_URL + "/");
const url ="flists" + "/" + username.value + "/" + id.value

const showContent = ref<string>()
const linkDecoration = ref<string>("text-as-anchor")
const contentShow = () => {
  showContent.value = flistPreview.value?.content.join("\n")
  linkDecoration.value = ""
  
}

onMounted(async () => {
  try {
    const encodedUrl = url.replaceAll("/", "%2F");
    flistPreview.value = (await api.get<FlistPreview>("/v1/api/fl/preview/" + encodedUrl)).data;
    flistPreview.value.content = flistPreview.value.content.slice(1)
    showContent.value = "show content on click"
    pending.value = false
  } catch (error: any) {
    toast.error(error.response?.data);
  }
});

</script>

<style scoped>
.Btn{
  position: relative;
  left: -18px;
  height: 40px;
  width: 110px;
  margin-left:0px;
}

.chip{
  height: 40px;
  position: relative;
  left: -11px;
}

.text-as-anchor {
  color: #42A5F5; 
  cursor: pointer;
}
.text-as-anchor:hover {
  text-decoration: underline;
}
</style>