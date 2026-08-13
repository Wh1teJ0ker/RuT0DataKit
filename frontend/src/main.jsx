import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider } from "antd";
import zhCN from "antd/locale/zh_CN";
import App from "./App";

// v1.2.0 T98：ConfigProvider 注入 theme token，为后续密度/暗色扩展预留入口。
// 当前仅设 colorPrimary 保持与 antd 默认一致，不引入 compact algorithm 避免全局样式突变。
ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <ConfigProvider
      locale={zhCN}
      theme={{ token: { colorPrimary: "#1677ff" } }}
    >
      <App />
    </ConfigProvider>
  </React.StrictMode>
);
