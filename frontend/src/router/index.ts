import { createRouter, createWebHistory, RouteRecordRaw } from "vue-router";
const Login = () => import("../components/Login.vue");
const CreateFlist = () => import("../components/CreateFlist.vue");
const Home = () => import("../components/Home.vue");
const UserFlist = () => import("../components/UserFlist.vue");
const PreviewFlist = () => import("../components/PreviewFlist.vue");

const routes: Array<RouteRecordRaw> = [
  {
    path: "/login",
    name: "login",
    component: Login,
  },
  {
    path: "/myflists",
    name: "myflists",
    component: UserFlist,
    meta: { requiresAuth: true },
  },
  {
    path: "/create",
    name: "create",
    component: CreateFlist,
    meta: { requiresAuth: true },
  },
  {
    path: "/flists/:username/:id",
    name: "previewflist",
    component: PreviewFlist,
  },
  {
    path: "/",
    name: "home",
    component: Home,
  },
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
});

router.beforeEach((to, _, next) => {
  const token: string | null = sessionStorage.getItem("token");
  console.log(token);
  console.log(to.meta.requiresAuth);
  if (to.meta.requiresAuth && (token == null || token.length == 0)) {
    console.log("ffej");
    next({ name: "login" });
  } else {
    next();
  }
});

export default router;
