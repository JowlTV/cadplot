import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri espera porta fixa e sem abrir navegador.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "es2022", sourcemap: false, outDir: "dist", emptyOutDir: true },
});
