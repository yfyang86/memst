/**
 * MemSt Frontend Configuration
 * 
 * This file provides configuration for the MemSt frontend.
 * It reads values from environment variables (Vite) with fallback defaults.
 * 
 * Configuration options:
 * - VITE_API_HOST: Backend API host (default: 127.0.0.1)
 * - VITE_API_PORT: Backend API port (default: 8193)
 * - VITE_DEV_PORT: Frontend dev server port (default: 3000)
 * 
 * For local overrides, create a .env.local file in memst-ui directory:
 * 
 * VITE_API_HOST=192.168.1.100
 * VITE_API_PORT=8193
 * VITE_DEV_PORT=3001
 */

// Read environment variables from Vite
const apiHost = import.meta.env.VITE_API_HOST || '127.0.0.1';
const apiPort = import.meta.env.VITE_API_PORT || '8193';
const devPort = parseInt(import.meta.env.VITE_DEV_PORT || '3000', 10);

// Configuration object
export const config = {
  api: {
    baseUrl: `http://${apiHost}:${apiPort}`,
    versionPath: "/api/v1",
  },
  dev: {
    port: devPort,
  },
};

/**
 * Get the full API base URL
 */
export function getApiBaseUrl(): string {
  return `${config.api.baseUrl}${config.api.versionPath}`;
}

// Log current config on load
console.log(`[MemSt] API Base URL: ${getApiBaseUrl()}`);
