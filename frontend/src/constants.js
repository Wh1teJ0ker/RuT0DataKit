// 前端单一真相源：版本号 + 开发状态文案。
//
// 版本号必须与以下位置保持一致（Tauri / Vite 不支持跨文件模板，需手动同步）：
//   - Cargo.toml                  workspace.package.version          （Rust crates 真相源）
//   - src-tauri/tauri.conf.json   version                            （打包元信息）
//   - frontend/package.json       version                            （npm 元信息）
export const APP_VERSION = "v1.1.2";

export const DEV_STATUS = "开发中";

/**
 * 表格默认每页行数（不含表头行）。
 *
 * 集中定义以避免 App.jsx / state/factory.js / DataTable.jsx 等多处硬编码 50。
 * 各消费方一律 `import { PAGE_SIZE } from "../constants"`，不在本地重复声明。
 */
export const PAGE_SIZE = 50;
