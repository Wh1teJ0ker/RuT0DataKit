// 前端单一真相源：版本号 + 开发状态文案。
//
// 版本号必须与以下位置保持一致（Tauri / Vite 不支持跨文件模板，需手动同步）：
//   - Cargo.toml                  workspace.package.version          （Rust crates 真相源）
//   - src-tauri/tauri.conf.json   version                            （打包元信息）
//   - frontend/package.json       version                            （npm 元信息）
// 所有 UI 文案统一从这里取，避免四处散落。
export const APP_VERSION = "v1.0.0";

// 后续版本业务能力的统一占位状态文案。
export const DEV_STATUS = "开发中";
