// 封装 Tauri v2 全局 API 调用。
// 保持 withGlobalTauri: true，不引入 @tauri-apps/api。
const { invoke } = window.__TAURI__.core;
const { save, open } = window.__TAURI__.dialog;

export const tauriInvoke = invoke;

// 保存对话框：defaultPath + filters（单个扩展名小写传入，name 转大写展示）
export const saveDialog = (defaultPath, ext) =>
  save({
    defaultPath,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  });

// 打开对话框：默认单选，filters 控制可选项；用户取消返回 null。
export const openDialog = (ext) =>
  open({
    multiple: false,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  });

// ─────────────────────────────────────────────────────────────────────
// T0-16 新增命令封装：列脱敏 + 校验 + 导出 + 规则集落盘。
// 参数名 camelCase，由 Tauri 自动转 snake_case 传到 Rust 端。
// ─────────────────────────────────────────────────────────────────────

// 对 selected_columns 中的列应用 rules.maskers，未勾选列原样。
export async function applyRulesCols(inputPath, rulesJson, selectedColumns) {
  return tauriInvoke("apply_rules_cols", { inputPath, rulesJson, selectedColumns });
}

// 跑校验 pipeline，返回 valid_matrix。
export async function runValidate(inputPath, rulesJson) {
  return tauriInvoke("run_validate", { inputPath, rulesJson });
}

// 应用列脱敏 → 投影列 → 可选按行索引过滤 → 写 CSV。
export async function exportRecordsCsv(
  inputPath,
  rulesJson,
  selectedColumns,
  columnOrder,
  selectedRowIndices,
  outPath
) {
  return tauriInvoke("export_records_csv", {
    inputPath,
    rulesJson,
    selectedColumns,
    columnOrder,
    selectedRowIndices,
    outPath,
  });
}

// 同 exportRecordsCsv，但写 XLSX。
export async function exportRecordsXlsx(
  inputPath,
  rulesJson,
  selectedColumns,
  columnOrder,
  selectedRowIndices,
  outPath
) {
  return tauriInvoke("export_records_xlsx", {
    inputPath,
    rulesJson,
    selectedColumns,
    columnOrder,
    selectedRowIndices,
    outPath,
  });
}

// 把 rules_json 序列化为 YAML 写盘。
export async function saveRuleset(rulesJson, outPath) {
  return tauriInvoke("save_ruleset", { rulesJson, outPath });
}

// 读取 YAML 规则集文件为字符串。前端用 js-yaml 解析为 RuleSet 对象。
export async function readRuleset(path) {
  return tauriInvoke("read_ruleset", { path });
}

// ─────────────────────────────────────────────────────────────────────
// T0-23 新增命令封装：规则试运行 + 算子类型清单（接 T0-22 后端命令）。
// 参数名 camelCase，由 Tauri 自动转 snake_case 传到 Rust 端。
// ─────────────────────────────────────────────────────────────────────

// 试运行脱敏规则（直接对用户输入值跑算子，不依赖任何文件）：
// apply_mask_op → { input, output }。
// 规则管理是独立系统，试运行不再读取已导入数据文件的首行。
export async function previewMaskRuleValue(input, masker, params) {
  return tauriInvoke("preview_mask_rule_value", { input, masker, params });
}

// 试运行校验规则（直接对用户输入值跑算子，不依赖任何文件）：
// apply_validate_op → { input, valid, message }。
export async function previewValidateRuleValue(
  input,
  validator,
  params,
  regex,
  message
) {
  return tauriInvoke("preview_validate_rule_value", {
    input,
    validator,
    params,
    regex,
    message,
  });
}

// 脱敏算子类型清单（通用算子 + 预置别名）：[{ name, label }, ...]。
export async function listMaskOpTypes() {
  return tauriInvoke("list_mask_op_types");
}

// 校验算子类型清单（通用算子 + 预置别名）：[{ name, label }, ...]。
export async function listValidateOpTypes() {
  return tauriInvoke("list_validate_op_types");
}
