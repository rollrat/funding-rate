import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
    plugins: [react()],
    server: {
        port: 3000,
        proxy: {
            '/snapshots': {
                target: 'http://localhost:12090',
                changeOrigin: true,
            },
        },
    },
})

