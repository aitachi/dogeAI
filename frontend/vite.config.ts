import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  base: '/new/',
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, './src')
    }
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'https://115.190.62.87',
        changeOrigin: true,
        secure: false
      },
      '/v1': {
        target: 'https://115.190.62.87',
        changeOrigin: true,
        secure: false
      }
    }
  },
  build: {
    outDir: '/var/www/aitachi.top/new',
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor': ['vue', 'vue-router', 'pinia'],
          'api': ['axios']
        }
      }
    },
    chunkSizeWarningLimit: 500
  }
})
