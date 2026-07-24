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

// 规则标签清单（静态三选一）：["extract","mask","validate"]。
// v0.4.4 规则引擎重构后，规则池初始为空，tag 为单值字段，三选一对应
// 「数据提取 / 数据脱敏 / 数据校验」三种用途。供 RulesView「按标签过滤」
// Select 使用。rulesJson 参数保留（向后兼容前端调用签名），当前实现不读取。
export async function listRuleTags(rulesJson) {
  return tauriInvoke("list_rule_tags", { rulesJson: rulesJson ?? null });
}

// v0.4.4 T11-3：拉取后端内置规则集（builtin_ruleset），启动时加载到
// state.rules（dispatch SET_RULES）。返回 { maskers: [], validators: [...] }，
// 与 extract_text 的 rules_json 路径同构。
export async function listBuiltinRules() {
  return tauriInvoke("list_builtin_rules");
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
// （v0.4.1 T6-5：模板生成已移除，下方保留接口注释作为历史参考。）

// ─────────────────────────────────────────────────────────────────────
// v0.4.1 T6-5：从自然语言描述构造正则，返回 {pattern, explanation, matched_clues}。
// 纯本地规则化推断，不调用网络。
// ─────────────────────────────────────────────────────────────────────
export async function regexConstruct(statement) {
  return tauriInvoke("regex_construct", { statement });
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

// ─────────────────────────────────────────────────────────────────────
// v0.4.2 T7-2 设置模块：tshark 多平台自动探测 + 路径配置持久化
// 参数名 camelCase，由 Tauri 自动转 snake_case 传到 Rust 端。
// ─────────────────────────────────────────────────────────────────────

// 自动探测本机可用的 tshark。返回 { path: string|null, version: string|null }。
// 探测顺序：用户已保存路径 → PATH 中 tshark → 各平台候选绝对路径。
// 仅本机子进程探测，不调用网络（docs/00 §6 「不外发数据」约束）。
export async function detectTshark() {
  return tauriInvoke("detect_tshark");
}

// 从 app_config_dir/settings.json 读取用户保存的 tshark 覆盖路径。
// 文件不存在 / 字段缺失返回 null。读到路径同时注入运行时，立即生效。
export async function loadTsharkPath() {
  return tauriInvoke("load_tshark_path");
}

// 把 tshark 覆盖路径保存到 app_config_dir/settings.json。
// path 为 string 时写入；为 null 时清除自定义路径，回退到 PATH。
// 同时注入运行时，立即生效（无需重启 app）。
export async function saveTsharkPath(path) {
  return tauriInvoke("save_tshark_path", { path });
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.3 T9-5 数据提取命令封装
// 参数名 camelCase，由 Tauri 自动转 snake_case 传到 Rust 端。
// ─────────────────────────────────────────────────────────────────────

// 从文本提取 PII，返回 { findings, counts }。
// rulesJson 为可选 RuleSet JSON 字符串，null/undefined 时后端走 builtin（phone/bankcard/ip）。
// v0.4.4 T10-3：新增 rulesJson 参数，前端从规则管理池勾选规则后构造。
export async function extractText(content, rulesJson) {
  return tauriInvoke("extract_text", { content, rulesJson });
}

// 从 .txt 文件提取 PII，返回 { findings, counts }。rulesJson 语义同 extractText。
export async function extractFile(path, rulesJson) {
  return tauriInvoke("extract_file", { path, rulesJson });
}

// 把 findings 按格式（txt/csv/json）导出到 outPath。
export async function exportExtract(findings, format, outPath) {
  return tauriInvoke("export_extract", {
    findingsJson: JSON.stringify(findings),
    format,
    outPath,
  });
}

// ─────────────────────────────────────────────────────────────────────
// v0.5.x RulesView 试运行：对单条样例值应用脱敏模版，返回脱敏结果。
// 不读文件、不落盘，供用户在作用于真实数据前验证参数。
// 返回 { ok: bool, masked: string|null, error: string|null }。
// ─────────────────────────────────────────────────────────────────────
export async function trialMask(scope, paramsJson, sampleValue) {
  return tauriInvoke("trial_mask", {
    scope,
    paramsJson,
    sampleValue,
  });
}
