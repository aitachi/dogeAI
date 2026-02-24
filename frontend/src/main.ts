import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'

// 样式
import './styles/main.css'

const app = createApp(App)

app.use(createPinia())
app.use(router)

// 初始化认证状态
import { useAuthStore } from './stores/auth'
import { useAdminAuthStore } from './stores/adminAuth'
const authStore = useAuthStore()
const adminAuthStore = useAdminAuthStore()
authStore.init()
adminAuthStore.init()

app.mount('#app')
