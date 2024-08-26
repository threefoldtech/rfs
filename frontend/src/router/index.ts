import { createRouter, createWebHistory, RouteRecordRaw } from "vue-router";
const Login = () => import("../components/Login.vue")
const CreateFlist = () => import("../components/CreateFlist.vue")
const Home = () => import("../components/Home.vue")
const UserFlist = () => import("../components/UserFlist.vue")
const FollowUp = () => import("../components/FollowUp.vue")
const PreviewFlist = () => import("../components/PreviewFlist.vue")


const routes: Array<RouteRecordRaw> = [
  {
    path: "/login",
    name: "Login",
    component: Login,
  },
  {
    path: "/flists",
    name: "Flists",
    component: UserFlist,
    meta: { requireAuth: true },
  },
  {
    path: "/follow_up/:id",
    name: "FollowUp",
    component: FollowUp,
    meta: { requireAuth: true },
  },
  {
    path: "/create",
    name: "Create",
    component: CreateFlist,
    meta: { requiresAuth: true },
  },
  {
    path: "/",
    name: "Home",
    component: Home,
  },
  {
    path: "/flists/:username/:id",
    name: "PreviewFlist",
    component: PreviewFlist,
  },
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
});

router.beforeEach((to, _, next) => {
  const token: string | null = sessionStorage.getItem("token");
  if (to.meta.requiresAuth && token === null) {
    next({ name: "Login" });
  } else {
    next();
  }
});

export default router;
