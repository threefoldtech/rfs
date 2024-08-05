import { createRouter, createWebHistory, RouteRecordRaw } from "vue-router";
import { Login } from "../components/Login.vue"


const routes: Array<RouteRecordRaw> = [
  {path:"/login", 
    name:"Login", 
    component:Login, 
    meta:{requiresAuth:true},}
  
];

const router = createRouter({
  history: createWebHistory(process.env.BASE_URL),
  routes,
});

router.beforeEach((to, from, next) => {
  const token: string | null = sessionStorage.getItem("token")
  if (to.meta.requiresAuth && token === null){
    next({name:"Login"})
  }else{
    next()
  }
});

export default router;