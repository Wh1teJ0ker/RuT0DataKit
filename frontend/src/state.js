// 全局 state + appReducer。
// 顶层 useReducer 持有，子组件通过 props 拿 state/dispatch；切 view 不丢 state。

export const PREVIEW_ROW_LIMIT = 200;

export const initialState = {
  // 文件
  filePath: null,
  sourceType: null, // "csv" | "xlsx"
  headers: [],
  rows: [], // 全部行（PREVIEW_ROW_LIMIT 截断用于预览）
  rowCount: 0,
  // v0.4.0 预处理归一化产物（PreprocessView 导入，mask/validate/export 复用）。
  // 与上面 filePath/headers/rows 平行存在：T5-7 起 MaskView/ValidateView 已切到
  // 以本字段为唯一数据源（旧 SET_FILE 路径保留兼容过渡，仅 export 等仍读文件
  // 路径的视图使用，T5-13 端到端时再决定是否清理）。
  // 切 view 不重置；导入新文件时整体覆盖。
  records: null, // { headers, rows, rowCount, sourceType } 或 null
  // 脱敏结果
  maskedRows: null, // null 表示未运行；[[...]] 表示脱敏后全行
  maskedSummary: null,
  // 校验结果
  validateResult: null, // { headers, rows, valid_matrix, summary }
  // 日志扫描结果（v0.2.0）：logEntries 为原始日志条目，logReport 为扫描报告。
  // 切到其他 view 再切回不丢数据（与 maskedRows/validateResult 同策略）。
  logEntries: [], // Vec<LogEntry>
  logReport: null, // Report 或 null
  logLoading: false,
  // 流量分析结果（v0.3.0）：pcapEntries 为原始 HTTP 请求，pcapReport 为扫描报告。
  // 与 log 状态同策略，跨视图不丢。
  pcapEntries: [],
  pcapReport: null,
  pcapLoading: false,
  // 规则（全局规则库）：RulesView 管理，独立于文件导入。
  // MaskView/ValidateView 不直接写入这里；脱敏/校验应用时用
  // maskOverrides/validateOverrides 优先 + 全局 rules 兜底合并。
  rules: { maskers: [], validators: [] }, // RuleSet 形状
  rulesTagFilter: null, // RulesView「按标签过滤」：null=全部，否则按 tag 过滤
  // MaskView/ValidateView 的「表头-算子映射」是会话级临时配置：
  // 不污染全局规则库，导入新文件时清空。
  // key=表头，value={masker/validator, params, description?}
  maskOverrides: {}, // { [header]: MaskRule }
  validateOverrides: {}, // { [header]: FieldRule }
  editingRule: null, // null=关闭；{ kind: "mask" | "validate", ...rule, __index? }=开启
                    // 新增态仅带 kind；编辑态带 __index + 完整 rule 字段。
  // 列勾选
  selectedColumns: [], // MaskView 用：要脱敏的列名
  columnOrder: [], // ExportView 用：列顺序
  exportColumns: [], // ExportView 用：要导出的列名子集
  exportFormat: "csv", // "csv" | "xlsx" | "json"
  exportSource: "masked", // ExportView 用：导出源 "masked" | "validate" | "records"（旧值 "raw" 作 "records" 别名，ExportView 内部归一）
  validateFilter: "all", // ExportView 用：校验后行过滤 "all" | "valid" | "invalid"
  // 导航
  activeView: "preprocess",
  // UI
  actionHint: "",
  loading: false,
  // T5-10/T5-12 Tools Tab 状态。toolsActiveTab：ToolsView 顶部下拉栏选中项
  // （"sql" | "regex"），切 view 不重置；切 Tab 也不清各自子状态。
  toolsActiveTab: "sql",
  // v0.4.1 T6-3：Sidebar Tools SubMenu 展开/折叠受控状态，默认 false（折叠）。
  sidebarToolsOpen: false,
  // T5-10 SQL 解析子界面状态
  sqlParseInput: "",
  sqlParseResult: null,
  // T5-12 正则解析子界面状态
  // regexSubTab：RegexTool 内部子 Tab（"explain" | "construct"），切 view 不重置。
  regexSubTab: "explain",
  // 解析 Tab
  regexExplainInput: "",
  regexExplainResult: null, // Vec<RegexTokenDesc> 或 null
  // 构造 Tab（v0.4.1 T6-5：自然语言描述 → 正则）
  regexConstructInput: "",
  regexConstructResult: null, // { pattern, explanation, matched_clues } 或 null
  // T5-6 搜索界面状态：searchMode 三选 "keyword" | "regex" | "exact_field"。
  // searchResults 为后端 SearchResult { hits: [{ row, col, field, value, snippet }] } 或 null。
  // searchElapsedMs 单次查询耗时（前端 performance.now 差值），命中数取 hits.length。
  // filteredRowIndices：从 SearchView 跳转到脱敏/导出 view 时携带的命中行号集合。
  searchMode: "keyword",
  searchKeywordInput: "", // 关键词模式输入（空格分隔多词）
  searchRegexInput: "", // 正则模式输入
  searchExactField: "", // 精确字段模式：字段名
  searchExactValue: "", // 精确字段模式：字段值
  searchKeywordMode: "and", // "and" | "or"
  searchResults: null, // SearchResult 或 null
  searchElapsedMs: null, // Number 或 null
  filteredRowIndices: null, // Number[] 或 null（脱敏/导出 view 据此过滤行）
  // v0.4.2 T7-3 设置模块状态：tshark 路径配置。
  // tsharkPath：用户保存的覆盖路径（null=用 PATH/auto）；持久化在 settings.json。
  // tsharkDetected：detect_tshark 命令返回的 { path, version } 或 null（未检测到）。
  // tsharkLoading：探测/保存进行中标志，禁用按钮防重入。
  tsharkPath: null,
  tsharkDetected: null,
  tsharkLoading: false,
  // v0.4.3 T9-5 数据提取模块状态。
  // extractMode："file"（文件导入）| "text"（文本粘贴）。
  // extractInput：文件模式下是路径字符串，文本模式下是粘贴内容。
  // extractResult：{ findings: [{type, value}], counts: {phone, bankcard, ip} } 或 null。
  // extractLoading：提取进行中标志，禁用按钮防重入。
  extractMode: "file",
  extractInput: "",
  extractResult: null,
  extractLoading: false,
};

function withRules(state, mutator) {
  // immutable 更新 rules.{maskers,validators} 的辅助。
  const next = {
    maskers: [...state.rules.maskers],
    validators: [...state.rules.validators],
  };
  mutator(next);
  return { ...state, rules: next };
}

export function appReducer(state, action) {
  switch (action.type) {
    case "SET_FILE": {
      const { filePath, sourceType, headers, rows, rowCount } = action;
      return {
        ...state,
        filePath,
        sourceType,
        headers,
        rows,
        rowCount,
        // 导入新文件时清空上一次脱敏 / 校验结果。
        maskedRows: null,
        maskedSummary: null,
        validateResult: null,
        // 会话级映射与具体文件绑定，导入新文件时清空（不污染全局规则库）。
        maskOverrides: {},
        validateOverrides: {},
        // 默认全选全部数据列，列顺序 = headers，导出列 = headers。
        selectedColumns: [...headers],
        columnOrder: [...headers],
        exportColumns: [...headers],
        exportFormat: "csv",
        actionHint: "",
        // v0.4.0 T6-1：SET_FILE 不再清空 records——顶部导入工具条已移除（T6-2），
        // SET_FILE 仅由内部流程保留；与 PreprocessView 走 SET_RECORDS 是两套并行的
        // 导入入口；用户痛点是经 SET_FILE 后 records 被清空导致 SearchView/ExportView
        // 读不到数据。此处保留 records 不动；若新文件确需覆盖 records，由调用方显式再
        // dispatch SET_RECORDS（PreprocessView 流程已经这么做）。
        // records 字段交由 SET_RECORDS 单独管理。
      };
    }
    case "SET_MASKED": {
      const { maskedRows, maskedSummary } = action;
      return { ...state, maskedRows, maskedSummary };
    }
    case "SET_VALIDATE": {
      const { validateResult } = action;
      return { ...state, validateResult };
    }
    case "SET_LOG_ENTRIES": {
      const { logEntries } = action;
      return { ...state, logEntries: logEntries || [] };
    }
    case "SET_LOG_REPORT": {
      const { logReport } = action;
      return { ...state, logReport };
    }
    case "SET_LOG_LOADING": {
      const { logLoading } = action;
      return { ...state, logLoading };
    }
    case "SET_PCAP_ENTRIES": {
      const { pcapEntries } = action;
      return { ...state, pcapEntries: pcapEntries || [] };
    }
    case "SET_PCAP_REPORT": {
      const { pcapReport } = action;
      return { ...state, pcapReport };
    }
    case "SET_PCAP_LOADING": {
      const { pcapLoading } = action;
      return { ...state, pcapLoading };
    }
    case "SET_RULES": {
      const { rules } = action;
      return {
        ...state,
        rules: {
          maskers: rules?.maskers ? [...rules.maskers] : [],
          validators: rules?.validators ? [...rules.validators] : [],
        },
      };
    }
    case "SET_RULES_TAG_FILTER": {
      const { tag } = action;
      return { ...state, rulesTagFilter: tag ?? null };
    }
    case "ADD_MASK_RULE": {
      const { rule } = action;
      return withRules(state, (r) => r.maskers.push(rule));
    }
    case "UPDATE_MASK_RULE": {
      const { index, rule } = action;
      return withRules(state, (r) => {
        if (index < 0 || index >= r.maskers.length) return;
        r.maskers[index] = rule;
      });
    }
    case "REMOVE_MASK_RULE": {
      const { index } = action;
      return withRules(state, (r) => {
        if (index < 0 || index >= r.maskers.length) return;
        r.maskers.splice(index, 1);
      });
    }
    case "ADD_VALIDATOR_RULE": {
      const { rule } = action;
      return withRules(state, (r) => r.validators.push(rule));
    }
    case "UPDATE_VALIDATOR_RULE": {
      const { index, rule } = action;
      return withRules(state, (r) => {
        if (index < 0 || index >= r.validators.length) return;
        r.validators[index] = rule;
      });
    }
    case "REMOVE_VALIDATOR_RULE": {
      const { index } = action;
      return withRules(state, (r) => {
        if (index < 0 || index >= r.validators.length) return;
        r.validators.splice(index, 1);
      });
    }
    case "SET_EDITING_MASK_RULE": {
      const { rule } = action;
      return { ...state, editingMaskRule: rule };
    }
    case "SET_EDITING_VALIDATOR_RULE": {
      const { rule } = action;
      return { ...state, editingValidatorRule: rule };
    }
    // ─────────────────────────────────────────────────────────────────────
    // T0-23 新增：单一 editingRule（合并 mask/validate）+ 通用 ADD/UPDATE/REMOVE
    // 按 action.kind 路由到 maskers / validators。旧 8 个 mask/validator action
    // 实现保留向后兼容（T0-24 视情况清理），不动。
    // ─────────────────────────────────────────────────────────────────────
    case "SET_EDITING_RULE": {
      const { rule } = action;
      return { ...state, editingRule: rule };
    }
    case "CLEAR_EDITING_RULE": {
      return { ...state, editingRule: null };
    }
    case "ADD_RULE": {
      const { kind, rule } = action;
      return withRules(state, (r) => {
        if (kind === "mask") r.maskers.push(rule);
        else if (kind === "validate") r.validators.push(rule);
      });
    }
    case "UPDATE_RULE": {
      const { kind, index, rule } = action;
      return withRules(state, (r) => {
        if (kind === "mask") {
          if (index >= 0 && index < r.maskers.length) r.maskers[index] = rule;
        } else if (kind === "validate") {
          if (index >= 0 && index < r.validators.length)
            r.validators[index] = rule;
        }
      });
    }
    case "REMOVE_RULE": {
      const { kind, index } = action;
      return withRules(state, (r) => {
        if (kind === "mask") {
          if (index >= 0 && index < r.maskers.length) r.maskers.splice(index, 1);
        } else if (kind === "validate") {
          if (index >= 0 && index < r.validators.length)
            r.validators.splice(index, 1);
        }
      });
    }
    case "SET_SELECTED_COLUMNS": {
      const { selectedColumns } = action;
      return { ...state, selectedColumns: [...selectedColumns] };
    }
    // ─────────────────────────────────────────────────────────────────────
    // 会话级「表头-算子」映射（maskOverrides / validateOverrides）：
    // 仅 MaskView/ValidateView 使用，不污染全局 rules。导入新文件时清空。
    // key=header，value=rule（masker/validator + params）。
    // ─────────────────────────────────────────────────────────────────────
    case "SET_MASK_OVERRIDE": {
      const { header, rule } = action;
      const next = { ...state.maskOverrides };
      if (rule == null) delete next[header];
      else next[header] = rule;
      return { ...state, maskOverrides: next };
    }
    case "CLEAR_MASK_OVERRIDE": {
      const { header } = action;
      const next = { ...state.maskOverrides };
      delete next[header];
      return { ...state, maskOverrides: next };
    }
    case "CLEAR_ALL_MASK_OVERRIDES": {
      return { ...state, maskOverrides: {} };
    }
    case "SET_VALIDATE_OVERRIDE": {
      const { header, rule } = action;
      const next = { ...state.validateOverrides };
      if (rule == null) delete next[header];
      else next[header] = rule;
      return { ...state, validateOverrides: next };
    }
    case "CLEAR_VALIDATE_OVERRIDE": {
      const { header } = action;
      const next = { ...state.validateOverrides };
      delete next[header];
      return { ...state, validateOverrides: next };
    }
    case "CLEAR_ALL_VALIDATE_OVERRIDES": {
      return { ...state, validateOverrides: {} };
    }
    case "SET_COLUMN_ORDER": {
      const { columnOrder } = action;
      return { ...state, columnOrder: [...columnOrder] };
    }
    case "SET_EXPORT_COLUMNS": {
      const { exportColumns } = action;
      return { ...state, exportColumns: [...exportColumns] };
    }
    case "SET_EXPORT_FORMAT": {
      const { exportFormat } = action;
      return { ...state, exportFormat };
    }
    case "SET_EXPORT_SOURCE": {
      const { exportSource } = action;
      return { ...state, exportSource };
    }
    case "SET_VALIDATE_FILTER": {
      const { validateFilter } = action;
      return { ...state, validateFilter };
    }
    case "SET_VIEW": {
      const { activeView } = action;
      return { ...state, activeView };
    }
    // ─────────────────────────────────────────────────────────────────────
    // T5-10/T5-12 Tools Tab 状态：顶部 Tab 切换 + SQL/正则子界面各自输入与结果。
    // 切 view 不重置；切 Tab 也不清各自子状态（保留用户已输入内容）。
    // ─────────────────────────────────────────────────────────────────────
    case "SET_TOOLS_ACTIVE_TAB": {
      const { toolsActiveTab } = action;
      return { ...state, toolsActiveTab };
    }
    case "SET_SIDEBAR_TOOLS_OPEN": {
      const { sidebarToolsOpen } = action;
      return { ...state, sidebarToolsOpen };
    }
    case "SET_SQL_PARSE_INPUT": {
      const { sqlParseInput } = action;
      return { ...state, sqlParseInput };
    }
    case "SET_SQL_PARSE_RESULT": {
      const { sqlParseResult } = action;
      return { ...state, sqlParseResult };
    }
    case "SET_REGEX_SUB_TAB": {
      const { regexSubTab } = action;
      return { ...state, regexSubTab };
    }
    case "SET_REGEX_EXPLAIN_INPUT": {
      const { regexExplainInput } = action;
      return { ...state, regexExplainInput };
    }
    case "SET_REGEX_EXPLAIN_RESULT": {
      const { regexExplainResult } = action;
      return { ...state, regexExplainResult };
    }
    case "SET_REGEX_CONSTRUCT_INPUT": {
      const { regexConstructInput } = action;
      return { ...state, regexConstructInput };
    }
    case "SET_REGEX_CONSTRUCT_RESULT": {
      const { regexConstructResult } = action;
      return { ...state, regexConstructResult };
    }
    // ─────────────────────────────────────────────────────────────────────
    // T5-6 搜索界面状态：查询输入 + 结果 + 耗时 + 跳转携带的行号集合。
    // ─────────────────────────────────────────────────────────────────────
    case "SET_SEARCH_MODE": {
      const { searchMode } = action;
      return { ...state, searchMode };
    }
    case "SET_SEARCH_KEYWORD_INPUT": {
      const { searchKeywordInput } = action;
      return { ...state, searchKeywordInput };
    }
    case "SET_SEARCH_REGEX_INPUT": {
      const { searchRegexInput } = action;
      return { ...state, searchRegexInput };
    }
    case "SET_SEARCH_EXACT_FIELD": {
      const { searchExactField } = action;
      return { ...state, searchExactField };
    }
    case "SET_SEARCH_EXACT_VALUE": {
      const { searchExactValue } = action;
      return { ...state, searchExactValue };
    }
    case "SET_SEARCH_KEYWORD_MODE": {
      const { searchKeywordMode } = action;
      return { ...state, searchKeywordMode };
    }
    case "SET_SEARCH_RESULTS": {
      const { searchResults, searchElapsedMs } = action;
      return { ...state, searchResults, searchElapsedMs };
    }
    case "SET_FILTERED_ROW_INDICES": {
      const { filteredRowIndices } = action;
      return { ...state, filteredRowIndices };
    }
    // v0.4.2 T7-3：tshark 设置相关状态。
    case "SET_TSHARK_PATH": {
      const { tsharkPath } = action;
      return { ...state, tsharkPath };
    }
    case "SET_TSHARK_DETECTED": {
      const { tsharkDetected } = action;
      return { ...state, tsharkDetected };
    }
    case "SET_TSHARK_LOADING": {
      const { tsharkLoading } = action;
      return { ...state, tsharkLoading };
    }
    // v0.4.3 T9-5 数据提取模块状态。
    case "SET_EXTRACT_MODE": {
      const { extractMode } = action;
      return { ...state, extractMode };
    }
    case "SET_EXTRACT_INPUT": {
      const { extractInput } = action;
      return { ...state, extractInput };
    }
    case "SET_EXTRACT_RESULT": {
      const { extractResult } = action;
      return { ...state, extractResult };
    }
    case "SET_EXTRACT_LOADING": {
      const { extractLoading } = action;
      return { ...state, extractLoading };
    }
    // v0.4.0 PreprocessView 导入产物：整体覆盖 records（headers/rows/rowCount/sourceType）。
    // 切 view 不重置；只在导入新文件时覆盖。mask/validate/export 后续从此读取。
    case "SET_RECORDS": {
      const { records } = action;
      return { ...state, records };
    }
    case "SET_HINT": {
      const { actionHint } = action;
      return { ...state, actionHint };
    }
    case "SET_LOADING": {
      const { loading } = action;
      return { ...state, loading };
    }
    case "RESET":
      return { ...initialState };
    default:
      return state;
  }
}
