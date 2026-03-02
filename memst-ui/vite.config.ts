import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig(({ mode }) => {
  // Load env variables - can also use VITE_API_HOST and VITE_API_PORT
  const env = loadEnv(mode, process.cwd(), '');
  
  const apiHost = env.VITE_API_HOST || '127.0.0.1';
  const apiPort = env.VITE_API_PORT || '8193';
  const target = `http://${apiHost}:${apiPort}`;

  return {
    plugins: [react()],
    server: {
      port: parseInt(env.VITE_DEV_PORT || '3000', 10),
      // Proxy API requests to backend
      // To change the backend URL:
      // 1. Create .env.local in memst-ui directory with VITE_API_HOST, VITE_API_PORT
      // 2. Or set environment variables when running: VITE_API_HOST=... npm run dev
      proxy: {
        '/api': {
          target,
          changeOrigin: true,
        },
      },
    },
  };
});
