// state.js barrel（T13 模块化后变薄）。
// 原 monolith（Sheet 工厂 / reducer / action 常量 / 14 个 dispatcher / hook）
// 按职责拆到 state/ 子模块，本文件仅做重导出，保持对外 import 路径不变。
//
// 子模块：
//   - state/constants.js  ACTION + initialState
//   - state/factory.js    createEmptySheet / createSheetFromImport
//   - state/reducer.js    reducer + patchActiveSheet
//   - state/AppContext.js Context + Provider + useAppContext hook

export { ACTION, initialState } from "./state/constants";
export {
  createEmptySheet,
  createSheetFromImport,
} from "./state/factory";
export { reducer, patchActiveSheet } from "./state/reducer";
export {
  AppContext,
  AppProvider,
  useAppContext,
} from "./state/AppContext.jsx";

// 向后兼容：原 useAppState 名称仍被 settings/cards/TsharkPathCard.jsx 使用。
// 语义等价于 useAppContext（需要 AppProvider 包裹）。
export { useAppContext as useAppState } from "./state/AppContext.jsx";
