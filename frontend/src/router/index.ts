import { createRouter, createWebHistory, RouteRecordRaw } from "vue-router";
import Login from "../components/Login.vue";
import CreateFlist from "../components/CreateFlist.vue";
import ViewFlists from "../components/ViewFlists.vue";
import FollowUp from "../components/FollowUp.vue";

const routes: Array<RouteRecordRaw> = [
  {
    path: "/login",
    name: "Login",
    component: Login,
  },
  {
    path: "/flists",
    name: "Flists",
    component: ViewFlists,
  },
  {
    path: "/follow",
    name: "Follow",
    component: FollowUp,
  },
  {
    path: "/create",
    name: "Create",
    component: CreateFlist,
    meta: { requiresAuth: true },
  },
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
});

// router.beforeEach((to, _, next) => {
//   const token: string | null = sessionStorage.getItem("token");
//   if (to.meta.requiresAuth && token === null) {
//     next({ name: "Login" });
//   } else {
//     next();
//   }
// });

export default router;
