// tauri/ barrel — v1.2.0 T97：按业务域拆分原 monolithic tauri.js（726 行 / ~30
// 导出函数）为 11 个子模块，本文件统一再导出，保持外部 `import { ... } from
// "../tauri"` 路径不变（17 个引用文件无需改动）。
//
// 子模块按业务域划分（每个文件只 import `@tauri-apps/api/core` 的 invoke，
// export.js 额外依赖 constants + sheet.getSheetData）：
//   ai / sheet / settings / mask / validate / extract / rules / undo / search
//   / columnOps / export

export {
  aiSuggest,
  invokeAiOp,
} from "./ai";

export {
  importFile,
  getSheetData,
  parseColumnAsJson,
} from "./sheet";

export {
  detectTshark,
  loadTsharkPath,
  saveTsharkPath,
  loadPageSize,
  savePageSize,
  checkUpdate,
  installUpdate,
} from "./settings";

export {
  maskColumn,
} from "./mask";

export {
  validateMultiRulesToTwoSheets,
} from "./validate";

export {
  extractValidateToNewSheet,
} from "./extract";

export {
  listRules,
  toggleRule,
  updateRuleParams,
  updateRuleTemplate,
  updateRuleExtractConfig,
} from "./rules";

export {
  undoOperation,
  redoOperation,
  listUndoableOperations,
} from "./undo";

export {
  searchRows,
  replaceAll,
} from "./search";

export {
  base64Column,
  hashColumn,
  transformColumn,
} from "./columnOps";

export {
  saveTextFile,
  toRowObjects,
  exportSheetToCsv,
  exportSheetToJson,
  exportSheetToTxt,
} from "./export";
