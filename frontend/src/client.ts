import axios from "axios";
import { useClipboard } from "@vueuse/core";
import { toast } from "vue3-toastify";
const { copy } = useClipboard();

export const api = axios.create({
  baseURL: import.meta.env.VITE_API_URL,
  headers: {
    "Content-Type": "application/json",
    Authorization: "Bearer " + sessionStorage.getItem("token"),
  },
});

export const copyLink = (url: string) => {
  copy(url);
  toast.success("Link Copied to Clipboard");
};
