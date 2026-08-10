import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Dev server 默认只监听 localhost，避免绑定所有接口（host: true）带来的
// 局域网暴露面。需要从其它设备/容器访问 dev server 时，显式设置环境变量
// `VITE_DEV_HOST=1`（任意非空值）即可恢复 `host: true` 行为。
// clearScreen=false 让 Tauri 的日志与 Vite 共存于同一终端。
const host = process.env.VITE_DEV_HOST ? true : "localhost";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    host,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
