// Tauri v2 invoke 封装层（v1.2.0 T97 拆分入口）。
//
// 原 monolithic 726 行 / ~30 函数按业务域拆为 11 个子模块（见 ./tauri/），
// 本文件仅再导出 barrel，保持外部 `import { ... } from "../tauri"` 路径不变
// （17 个引用文件无需改动）。
//
// 历史版本：
// v1.0.0：AI 占位契约 `aiSuggest` / `invokeAiOp`；导入流 `importFile` /
//   `getSheetData`；tshark 设置 `detectTshark` / `loadTsharkPath` /
//   `saveTsharkPath`；更新检查 `checkUpdate` / `installUpdate`。
// v1.0.0（导出面板重构）：`toRowObjects` 纯函数 + `fetchAllRowsForExport`
//   全表拉取（修复只导当前页的 BUG），CSV/JSON/TXT 导出函数接收 options
//   （分隔符/表头/模板/行尾/NDJSON 等）+ 内部拉全表。
// v1.1.0：新增数据处理原型 IPC（脱敏 / 校验 / 提取 / 规则管理）。
// v1.2.0 T97：拆分子模块，本文件改为 barrel 再导出。

export * from "./tauri/index";
