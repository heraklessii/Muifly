/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri dev sunucusunu sabit portta bekliyor; port doluysa hata vermeli ki
// pencere yanlis bir adrese baglanmasin.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],

  // Tauri CLI'in urettigi hatalari gizlememek icin.
  clearScreen: false,

  // Arayuz testleri: jsdom + Testing Library. Testler gercek backend'e
  // dokunmuyor; `src/lib/api.ts` her testte mock'lanıyor.
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.test.{ts,tsx}"],
    restoreMocks: true,
    // restoreMocks gerceklemeyi geri aliyor ama cagri gecmisini birakiyor;
    // o yuzden "kac kez cagrildi" iddialari testler arasinda sizardi.
    clearMocks: true,
  },

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // Rust tarafi degisince Vite bosuna yeniden derlemesin.
      ignored: ["**/src-tauri/**"],
    },
  },
});
