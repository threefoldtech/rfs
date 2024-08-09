import { Ref } from "vue";

export interface User {
  username: string;
  password: string;
}

export interface LoggedInUser {
  loggedInUser: Ref<string>;
  updateLoggedInUser: (user: string) => void;
}
