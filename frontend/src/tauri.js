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

// ─────────────────────────────────────────────────────────────────────
// v0.4.0 T5-7 records-based 脱敏 / 校验命令封装
// ─────────────────────────────────────────────────────────────────────
// 入参不再走文件路径，而是直接传 PreprocessView 归一化产出的
// headers + rows，避免 MaskView/ValidateView 再读盘。参数名 camelCase，
// 由 Tauri 自动转 snake_case 传到 Rust 端。

// 对 selected_columns 中的列应用 rules.maskers，未勾选列原样。
// 返回 { headers, masked_rows, summary, skipped_fields }，与 applyRulesCols 一致。
export async function applyRulesColsRecords(
  headers,
  rows,
  rulesJson,
  selectedColumns
) {
  return tauriInvoke("apply_rules_cols_records", {
    headers,
    rows,
    rulesJson,
    selectedColumns,
  });
}

// 跑校验 pipeline，返回 { headers, rows, valid_matrix, summary }，
// 与 runValidate 一致。
export async function runValidateRecords(headers, rows, rulesJson) {
  return tauriInvoke("run_validate_records", {
    headers,
    rows,
    rulesJson,
  });
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

// 同 exportRecordsCsv，但写 JSON（每行扁平对象，headers 做 key）。
export async function exportRecordsJson(
  inputPath,
  rulesJson,
  selectedColumns,
  columnOrder,
  selectedRowIndices,
  outPath
) {
  return tauriInvoke("export_records_json", {
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

// 规则可选标签并集（预置标签 ∪ 当前 ruleset 出现过的 tag）：
// 供 RuleDrawer 的 tags Select 下拉源与 RulesView「按标签过滤」共用。
// rulesJson 为当前编辑态的 RuleSet JSON 序列化（可为 null/空串，退化为仅预置标签）。
export async function listRuleTags(rulesJson) {
  return tauriInvoke("list_rule_tags", { rulesJson: rulesJson ?? null });
}

// ─────────────────────────────────────────────────────────────────────
// T2-5 新增命令封装：日志扫描（v0.2.0）
// 参数名 camelCase，由 Tauri 自动转 snake_case 传到 Rust 端。
// ─────────────────────────────────────────────────────────────────────

// 读取 .log 并跑日志扫描 pipeline，一次返回 { entries, report }。
//   - entries：LogEntry 数组，前端用于原始日志表渲染。
//   - report：含 findings + summary（total_lines/sqli_hits/.../top_attack_ips）。
export async function scanLogFile(path) {
  return tauriInvoke("scan_log_file", { path });
}

// ─────────────────────────────────────────────────────────────────────
// T3-4 新增命令封装：流量分析（v0.3.0）
// 参数名 camelCase，由 Tauri 自动转 snake_case 传到 Rust 端。
// ─────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────
// v0.4.0 数据预处理归一化命令（T5-2 提供）
// 参数名 camelCase，由 Tauri 自动转 snake_case 传到 Rust 端。
// ─────────────────────────────────────────────────────────────────────

// 读取任意支持格式（csv/xlsx/sql/json/pcap/log）统一归一为 Records。
// 返回 { headers, rows, source_type, row_count }，与 load_preview 形态对齐。
// PreprocessView 入口；后续 mask/validate/export 复用同一份 records。
export async function preprocessFile(path) {
  return tauriInvoke("preprocess_file", { path });
}

// 读取 .pcap/.pcapng 并跑流量扫描 pipeline，一次返回 { entries, report }。
//   - entries：HttpRequest 数组，前端用于原始 HTTP 请求表渲染。
//   - report：含 findings + summary（total_requests/sensitive_hits/
//     decoded_fragments/top_src_ips）。tshark 缺失时抛错，前端弹提示。
export async function scanPcapFile(path) {
  return tauriInvoke("scan_pcap_file", { path });
}

// ─────────────────────────────────────────────────────────────────────
// T5-11/T5-12 新增命令封装：正则解释 / 模板生成 / 模板清单（v0.4.0 Tools Tab）
// 参数名 camelCase，由 Tauri 自动转 snake_case 传到 Rust 端。
// ─────────────────────────────────────────────────────────────────────

// 解释一条正则字符串，返回 RegexTokenDesc 数组：
//   { token, kind, description, position }，kind ∈
//   literal/char_class/quantifier/anchor/group/backref/assertion/escape/unsupported。
// 非法正则返回 Err(String)，前端 message.error 提示。
export async function explainRegex(pattern) {
  return tauriInvoke("explain_regex", { pattern });
}

// 按模板名 + 参数生成正则字符串。params 为 HashMap<String,String> 形态的
// 普通对象，键名对应模板 params_schema.key。模板名见 list_regex_templates。
export async function generateRegex(templateName, params) {
  return tauriInvoke("generate_regex", {
    templateName,
    params: params || {},
  });
}

// 列出预置正则模板（email/phone_cn/idcard_cn/ipv4/url/sql_injection_*/mac）：
//   [{ name, description, params_schema: [{ key, description, required, default }] }]
export async function listRegexTemplates() {
  return tauriInvoke("list_regex_templates");
}

// ─────────────────────────────────────────────────────────────────────
// T5-9/T5-10 新增命令封装：SQL 探针序列解析（v0.4.0 Tools Tab）
// ─────────────────────────────────────────────────────────────────────

// 解析多行 SQL 探针序列，返回 SqlParseResult（还原数据库 + 探针明细）。
// inputs 为 SqlParseInput 数组，每行一条。
export async function parseSqlTool(inputs) {
  return tauriInvoke("parse_sql_tool", { inputs });
}

// ─────────────────────────────────────────────────────────────────────
// T5-5/T5-6 新增命令封装：搜索（v0.4.0 搜索界面）
// ─────────────────────────────────────────────────────────────────────

// 对一组 records 执行搜索。queryJson 为 SearchQuery 的 JSON 字符串：
//   - {"kind":"keyword","terms":["a","b"],"mode":"and"|"or"}
//   - {"kind":"regex","pattern":"..."}
//   - {"kind":"exact_field","field":"name","value":"Alice"}
// 返回 SearchResult { hits: [{ row, col, field, value, snippet }] }。
export async function searchRecords(headers, rows, queryJson) {
  return tauriInvoke("search_records", {
    headers,
    rows,
    queryJson,
  });
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.1 T6-4 SQL 盲注探针特征检测
// ─────────────────────────────────────────────────────────────────────

// 扫描预处理后的 records 找 SQL 盲注探针特征行。
// 返回 { detected: bool, samples: string[] }，samples 上限 50。
// 仅本地正则匹配，不调用网络（docs/00 §6 「不外发数据」约束）。
export async function detectSqlBlindFeatures(headers, rows) {
  return tauriInvoke("detect_sql_blind_features", {
    headers,
    rows,
  });
}
