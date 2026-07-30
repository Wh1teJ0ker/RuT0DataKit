import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri v2 习惯：host=true 便于移动设备/容器内访问 dev server；
// clearScreen=false 让 Tauri 的日志与 Vite 共存于同一终端。
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    host: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
