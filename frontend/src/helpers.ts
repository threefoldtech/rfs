import { useClipboard } from "@vueuse/core";
import { toast } from "vue3-toastify";
const { copy } = useClipboard();

export const copyLink = (url: string) => {
  copy(url);
  toast.success("Link Copied to Clipboard");
};
