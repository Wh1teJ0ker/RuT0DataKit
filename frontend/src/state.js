// 全局 state + appReducer。
// 顶层 useReducer 持有，子组件通过 props 拿 state/dispatch；切 view 不丢 state。
//
// v0.5.0 T12-5：原单 switch 全平铺 54 action 按业务领域切片为 13 个领域子函数
// （fileDomain / maskDomain / validateDomain / logDomain / pcapDomain / rulesDomain
// / searchDomain / toolsDomain / extractDomain / settingsDomain / exportDomain
// / navDomain / uiDomain），主 appReducer 顺序分派，每个领域函数对不相关
// action 返回 state 原样。action.type 抽 ACTION 常量对象，字符串值与原字面量
// 逐字符一致，组件 dispatch({ type: "SET_FILE", ... }) 调用零改动。SET_FILE 的
// 级联清空逻辑（maskedRows/validateResult/maskOverrides/... 等）保留在
// fileDomain 内集中处理，不外移，避免改组件语义。

export const PREVIEW_ROW_LIMIT = 200;

// ACTION 常量对象：value 与现有组件 dispatch 字符串字面量逐字符一致。
// 组件继续用字符串字面量 dispatch（不强制改用 ACTION.*），此处常量供后续
// 渐进迁移与 lint 校验使用。新增 action 时务必同时在此登记。
export const ACTION = {
  // file
  FILE_SET: "SET_FILE",
  RECORDS_SET: "SET_RECORDS",
  // mask
  MASKED_SET: "SET_MASKED",
  MASK_OVERRIDE_SET: "SET_MASK_OVERRIDE",
  MASK_OVERRIDE_CLEAR: "CLEAR_MASK_OVERRIDE",
  MASK_OVERRIDE_CLEAR_ALL: "CLEAR_ALL_MASK_OVERRIDES",
  // validate
  VALIDATE_SET: "SET_VALIDATE",
  VALIDATE_OVERRIDE_SET: "SET_VALIDATE_OVERRIDE",
  VALIDATE_OVERRIDE_CLEAR: "CLEAR_VALIDATE_OVERRIDE",
  VALIDATE_OVERRIDE_CLEAR_ALL: "CLEAR_ALL_VALIDATE_OVERRIDES",
  // log
  LOG_ENTRIES_SET: "SET_LOG_ENTRIES",
  LOG_REPORT_SET: "SET_LOG_REPORT",
  LOG_LOADING_SET: "SET_LOG_LOADING",
  // pcap
  PCAP_ENTRIES_SET: "SET_PCAP_ENTRIES",
  PCAP_REPORT_SET: "SET_PCAP_REPORT",
  PCAP_LOADING_SET: "SET_PCAP_LOADING",
  // rules
  RULES_SET: "SET_RULES",
  RULES_TAG_FILTER_SET: "SET_RULES_TAG_FILTER",
  // export / 列勾选
  SELECTED_COLUMNS_SET: "SET_SELECTED_COLUMNS",
  COLUMN_ORDER_SET: "SET_COLUMN_ORDER",
  EXPORT_COLUMNS_SET: "SET_EXPORT_COLUMNS",
  EXPORT_FORMAT_SET: "SET_EXPORT_FORMAT",
  EXPORT_SOURCE_SET: "SET_EXPORT_SOURCE",
  VALIDATE_FILTER_SET: "SET_VALIDATE_FILTER",
  // nav
  VIEW_SET: "SET_VIEW",
  // tools
  TOOLS_ACTIVE_TAB_SET: "SET_TOOLS_ACTIVE_TAB",
  SIDEBAR_TOOLS_OPEN_SET: "SET_SIDEBAR_TOOLS_OPEN",
  SQL_PARSE_INPUT_SET: "SET_SQL_PARSE_INPUT",
  SQL_PARSE_RESULT_SET: "SET_SQL_PARSE_RESULT",
  REGEX_SUB_TAB_SET: "SET_REGEX_SUB_TAB",
  REGEX_EXPLAIN_INPUT_SET: "SET_REGEX_EXPLAIN_INPUT",
  REGEX_EXPLAIN_RESULT_SET: "SET_REGEX_EXPLAIN_RESULT",
  REGEX_CONSTRUCT_INPUT_SET: "SET_REGEX_CONSTRUCT_INPUT",
  REGEX_CONSTRUCT_RESULT_SET: "SET_REGEX_CONSTRUCT_RESULT",
  // encrypt（T15-2）
  ENCRYPT_ALGO_SET: "SET_ENCRYPT_ALGO",
  ENCRYPT_MODE_SET: "SET_ENCRYPT_MODE",
  ENCRYPT_KEY_SET: "SET_ENCRYPT_KEY",
  ENCRYPT_INPUT_SET: "SET_ENCRYPT_INPUT",
  ENCRYPT_RESULT_SET: "SET_ENCRYPT_RESULT",
  ENCRYPT_COLUMNS_RESULT_SET: "SET_ENCRYPT_COLUMNS_RESULT",
  ENCRYPT_LOADING_SET: "SET_ENCRYPT_LOADING",
  // search
  SEARCH_MODE_SET: "SET_SEARCH_MODE",
  SEARCH_KEYWORD_INPUT_SET: "SET_SEARCH_KEYWORD_INPUT",
  SEARCH_REGEX_INPUT_SET: "SET_SEARCH_REGEX_INPUT",
  SEARCH_EXACT_FIELD_SET: "SET_SEARCH_EXACT_FIELD",
  SEARCH_EXACT_VALUE_SET: "SET_SEARCH_EXACT_VALUE",
  SEARCH_KEYWORD_MODE_SET: "SET_SEARCH_KEYWORD_MODE",
  SEARCH_RESULTS_SET: "SET_SEARCH_RESULTS",
  FILTERED_ROW_INDICES_SET: "SET_FILTERED_ROW_INDICES",
  // settings
  TSHARK_PATH_SET: "SET_TSHARK_PATH",
  TSHARK_DETECTED_SET: "SET_TSHARK_DETECTED",
  TSHARK_LOADING_SET: "SET_TSHARK_LOADING",
  // extract
  EXTRACT_MODE_SET: "SET_EXTRACT_MODE",
  EXTRACT_INPUT_SET: "SET_EXTRACT_INPUT",
  EXTRACT_RESULT_SET: "SET_EXTRACT_RESULT",
  EXTRACT_LOADING_SET: "SET_EXTRACT_LOADING",
  EXTRACT_SELECTED_INDICES_SET: "SET_EXTRACT_SELECTED_INDICES",
  EXTRACT_RULE_TAG_FILTER_SET: "SET_EXTRACT_RULE_TAG_FILTER",
  // ui
  HINT_SET: "SET_HINT",
  LOADING_SET: "SET_LOADING",
  RESET: "RESET",
};

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
  // extractResult：{ findings: [{type, value}], counts: {type: count, ...} } 或 null。
  //   v0.4.4：counts 改为动态聚合（后端 BTreeMap），不再硬编码 phone/bankcard/ip。
  // extractLoading：提取进行中标志，禁用按钮防重入。
  // extractSelectedIndices：勾选要应用的规则在 state.rules.validators 中的下标数组（v0.4.4）。
  // extractRuleTagFilter：extract 本地 tag 过滤，null=全部（v0.4.4）。
  extractMode: "file",
  extractInput: "",
  extractResult: null,
  extractLoading: false,
  extractSelectedIndices: [],
  extractRuleTagFilter: null,
  // v0.6.0 T15-2 加密 / 解密子界面状态（Tools Tab 第三项）。
  // encryptAlgo：算法 "aes" | "base64" | "hex"；encryptMode：模式 "encrypt" | "decrypt"。
  // encryptKey：AES 密钥（base64/hex 忽略）；encryptInput：单值试运行输入。
  // encryptResult：单值结果 { ok, result, error } 或 null。
  // encryptColumnsResult：批量列结果 { headers, processed_rows, summary, skipped_fields } 或 null。
  // encryptLoading：运行中标志，禁用按钮防重入。切 view 不重置；切 Tab 不清。
  encryptAlgo: "aes",
  encryptMode: "encrypt",
  encryptKey: "",
  encryptInput: "",
  encryptResult: null,
  encryptColumnsResult: null,
  encryptLoading: false,
};

// ─────────────────────────────────────────────────────────────────────
// 领域子 reducer：每个函数只处理本领域 action，其他 action 返回 state 原样。
// 顺序分派，后一个领域拿到前一个领域返回的 state。
// ─────────────────────────────────────────────────────────────────────

// 文件导入：SET_FILE（含级联清空）+ SET_RECORDS。
// SET_FILE 的级联清空（maskedRows/validateResult/maskOverrides/列配置/...
// 等）集中在此处理，不外移到各领域函数，避免改组件语义。
const fileDomain = (state, action) => {
  switch (action.type) {
    case ACTION.FILE_SET: {
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
    // v0.4.0 PreprocessView 导入产物：整体覆盖 records（headers/rows/rowCount/sourceType）。
    // 切 view 不重置；只在导入新文件时覆盖。mask/validate/export 后续从此读取。
    // v0.5.x 修复：SET_RECORDS 是 PreprocessView（主导入入口）的唯一路径，
    // 必须像 SET_FILE 一样级联初始化列配置（columnOrder/exportColumns/
    // selectedColumns）并清空上一次脱敏/校验结果与会话级映射，否则 ExportView
    // 因 columnOrder 为空而显示空白 Table。
    case ACTION.RECORDS_SET: {
      const { records } = action;
      const headers = records?.headers || [];
      // v0.5.x 修复：PreprocessView 是主导入入口，但旧 ExportView 仍从
      // state.filePath 取后端导出 inputPath。若 SET_RECORDS 不回填 filePath，
      // 导出会传 null → "invalid type: null, expected a string"。此处把
      // records.filePath 同步到顶层 filePath（records 对象内也保留一份）。
      const filePath = records?.filePath ?? null;
      return {
        ...state,
        records,
        filePath,
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
        actionHint: "",
      };
    }
    default:
      return state;
  }
};

// 脱敏结果 + 会话级表头-脱敏算子映射。
const maskDomain = (state, action) => {
  switch (action.type) {
    case ACTION.MASKED_SET: {
      const { maskedRows, maskedSummary } = action;
      return { ...state, maskedRows, maskedSummary };
    }
    // ─────────────────────────────────────────────────────────────────────
    // 会话级「表头-算子」映射（maskOverrides）：
    // 仅 MaskView 使用，不污染全局 rules。导入新文件时清空。
    // key=header，value=rule（masker + params）。
    // ─────────────────────────────────────────────────────────────────────
    case ACTION.MASK_OVERRIDE_SET: {
      const { header, rule } = action;
      const next = { ...state.maskOverrides };
      if (rule == null) delete next[header];
      else next[header] = rule;
      return { ...state, maskOverrides: next };
    }
    case ACTION.MASK_OVERRIDE_CLEAR: {
      const { header } = action;
      const next = { ...state.maskOverrides };
      delete next[header];
      return { ...state, maskOverrides: next };
    }
    case ACTION.MASK_OVERRIDE_CLEAR_ALL: {
      return { ...state, maskOverrides: {} };
    }
    default:
      return state;
  }
};

// 校验结果 + 会话级表头-校验算子映射。
const validateDomain = (state, action) => {
  switch (action.type) {
    case ACTION.VALIDATE_SET: {
      const { validateResult } = action;
      return { ...state, validateResult };
    }
    // ─────────────────────────────────────────────────────────────────────
    // 会话级「表头-算子」映射（validateOverrides）：
    // 仅 ValidateView 使用，不污染全局 rules。导入新文件时清空。
    // key=header，value=rule（validator + params）。
    // ─────────────────────────────────────────────────────────────────────
    case ACTION.VALIDATE_OVERRIDE_SET: {
      const { header, rule } = action;
      const next = { ...state.validateOverrides };
      if (rule == null) delete next[header];
      else next[header] = rule;
      return { ...state, validateOverrides: next };
    }
    case ACTION.VALIDATE_OVERRIDE_CLEAR: {
      const { header } = action;
      const next = { ...state.validateOverrides };
      delete next[header];
      return { ...state, validateOverrides: next };
    }
    case ACTION.VALIDATE_OVERRIDE_CLEAR_ALL: {
      return { ...state, validateOverrides: {} };
    }
    default:
      return state;
  }
};

// 日志扫描（v0.2.0）：logEntries 原始条目 + logReport 报告 + loading。
const logDomain = (state, action) => {
  switch (action.type) {
    case ACTION.LOG_ENTRIES_SET: {
      const { logEntries } = action;
      return { ...state, logEntries: logEntries || [] };
    }
    case ACTION.LOG_REPORT_SET: {
      const { logReport } = action;
      return { ...state, logReport };
    }
    case ACTION.LOG_LOADING_SET: {
      const { logLoading } = action;
      return { ...state, logLoading };
    }
    default:
      return state;
  }
};

// 流量分析（v0.3.0）：pcapEntries 原始请求 + pcapReport 报告 + loading。
const pcapDomain = (state, action) => {
  switch (action.type) {
    case ACTION.PCAP_ENTRIES_SET: {
      const { pcapEntries } = action;
      return { ...state, pcapEntries: pcapEntries || [] };
    }
    case ACTION.PCAP_REPORT_SET: {
      const { pcapReport } = action;
      return { ...state, pcapReport };
    }
    case ACTION.PCAP_LOADING_SET: {
      const { pcapLoading } = action;
      return { ...state, pcapLoading };
    }
    default:
      return state;
  }
};

// 全局规则库（RulesView 管理）：rules 全集 + tag 过滤。
const rulesDomain = (state, action) => {
  switch (action.type) {
    case ACTION.RULES_SET: {
      const { rules } = action;
      return {
        ...state,
        rules: {
          maskers: rules?.maskers ? [...rules.maskers] : [],
          validators: rules?.validators ? [...rules.validators] : [],
        },
      };
    }
    case ACTION.RULES_TAG_FILTER_SET: {
      const { tag } = action;
      return { ...state, rulesTagFilter: tag ?? null };
    }
    default:
      return state;
  }
};

// 列勾选 / 导出配置（MaskView 选列 + ExportView 列序/导出列/格式/源/校验过滤）。
const exportDomain = (state, action) => {
  switch (action.type) {
    case ACTION.SELECTED_COLUMNS_SET: {
      const { selectedColumns } = action;
      return { ...state, selectedColumns: [...selectedColumns] };
    }
    case ACTION.COLUMN_ORDER_SET: {
      const { columnOrder } = action;
      return { ...state, columnOrder: [...columnOrder] };
    }
    case ACTION.EXPORT_COLUMNS_SET: {
      const { exportColumns } = action;
      return { ...state, exportColumns: [...exportColumns] };
    }
    case ACTION.EXPORT_FORMAT_SET: {
      const { exportFormat } = action;
      return { ...state, exportFormat };
    }
    case ACTION.EXPORT_SOURCE_SET: {
      const { exportSource } = action;
      return { ...state, exportSource };
    }
    case ACTION.VALIDATE_FILTER_SET: {
      const { validateFilter } = action;
      return { ...state, validateFilter };
    }
    default:
      return state;
  }
};

// 导航：activeView 切换（切 view 不丢 state，所有领域状态保留）。
const navDomain = (state, action) => {
  switch (action.type) {
    case ACTION.VIEW_SET: {
      const { activeView } = action;
      return { ...state, activeView };
    }
    default:
      return state;
  }
};

// T5-10/T5-12 Tools Tab 状态：顶部 Tab 切换 + SQL/正则子界面各自输入与结果。
// 切 view 不重置；切 Tab 也不清各自子状态（保留用户已输入内容）。
const toolsDomain = (state, action) => {
  switch (action.type) {
    case ACTION.TOOLS_ACTIVE_TAB_SET: {
      const { toolsActiveTab } = action;
      return { ...state, toolsActiveTab };
    }
    case ACTION.SIDEBAR_TOOLS_OPEN_SET: {
      const { sidebarToolsOpen } = action;
      return { ...state, sidebarToolsOpen };
    }
    case ACTION.SQL_PARSE_INPUT_SET: {
      const { sqlParseInput } = action;
      return { ...state, sqlParseInput };
    }
    case ACTION.SQL_PARSE_RESULT_SET: {
      const { sqlParseResult } = action;
      return { ...state, sqlParseResult };
    }
    case ACTION.REGEX_SUB_TAB_SET: {
      const { regexSubTab } = action;
      return { ...state, regexSubTab };
    }
    case ACTION.REGEX_EXPLAIN_INPUT_SET: {
      const { regexExplainInput } = action;
      return { ...state, regexExplainInput };
    }
    case ACTION.REGEX_EXPLAIN_RESULT_SET: {
      const { regexExplainResult } = action;
      return { ...state, regexExplainResult };
    }
    case ACTION.REGEX_CONSTRUCT_INPUT_SET: {
      const { regexConstructInput } = action;
      return { ...state, regexConstructInput };
    }
    case ACTION.REGEX_CONSTRUCT_RESULT_SET: {
      const { regexConstructResult } = action;
      return { ...state, regexConstructResult };
    }
    // v0.6.0 T15-2 加密 / 解密子界面状态。
    case ACTION.ENCRYPT_ALGO_SET: {
      const { encryptAlgo } = action;
      return { ...state, encryptAlgo };
    }
    case ACTION.ENCRYPT_MODE_SET: {
      const { encryptMode } = action;
      return { ...state, encryptMode };
    }
    case ACTION.ENCRYPT_KEY_SET: {
      const { encryptKey } = action;
      return { ...state, encryptKey };
    }
    case ACTION.ENCRYPT_INPUT_SET: {
      const { encryptInput } = action;
      return { ...state, encryptInput };
    }
    case ACTION.ENCRYPT_RESULT_SET: {
      const { encryptResult } = action;
      return { ...state, encryptResult };
    }
    case ACTION.ENCRYPT_COLUMNS_RESULT_SET: {
      const { encryptColumnsResult } = action;
      return { ...state, encryptColumnsResult };
    }
    case ACTION.ENCRYPT_LOADING_SET: {
      const { encryptLoading } = action;
      return { ...state, encryptLoading };
    }
    default:
      return state;
  }
};

// T5-6 搜索界面状态：查询输入 + 结果 + 耗时 + 跳转携带的行号集合。
const searchDomain = (state, action) => {
  switch (action.type) {
    case ACTION.SEARCH_MODE_SET: {
      const { searchMode } = action;
      return { ...state, searchMode };
    }
    case ACTION.SEARCH_KEYWORD_INPUT_SET: {
      const { searchKeywordInput } = action;
      return { ...state, searchKeywordInput };
    }
    case ACTION.SEARCH_REGEX_INPUT_SET: {
      const { searchRegexInput } = action;
      return { ...state, searchRegexInput };
    }
    case ACTION.SEARCH_EXACT_FIELD_SET: {
      const { searchExactField } = action;
      return { ...state, searchExactField };
    }
    case ACTION.SEARCH_EXACT_VALUE_SET: {
      const { searchExactValue } = action;
      return { ...state, searchExactValue };
    }
    case ACTION.SEARCH_KEYWORD_MODE_SET: {
      const { searchKeywordMode } = action;
      return { ...state, searchKeywordMode };
    }
    case ACTION.SEARCH_RESULTS_SET: {
      const { searchResults, searchElapsedMs } = action;
      return { ...state, searchResults, searchElapsedMs };
    }
    case ACTION.FILTERED_ROW_INDICES_SET: {
      const { filteredRowIndices } = action;
      return { ...state, filteredRowIndices };
    }
    default:
      return state;
  }
};

// v0.4.2 T7-3 设置模块状态：tshark 路径配置 + 探测结果 + loading。
const settingsDomain = (state, action) => {
  switch (action.type) {
    case ACTION.TSHARK_PATH_SET: {
      const { tsharkPath } = action;
      return { ...state, tsharkPath };
    }
    case ACTION.TSHARK_DETECTED_SET: {
      const { tsharkDetected } = action;
      return { ...state, tsharkDetected };
    }
    case ACTION.TSHARK_LOADING_SET: {
      const { tsharkLoading } = action;
      return { ...state, tsharkLoading };
    }
    default:
      return state;
  }
};

// v0.4.3 T9-5 数据提取模块状态（v0.4.4 增规则选择 + tag 过滤）。
const extractDomain = (state, action) => {
  switch (action.type) {
    case ACTION.EXTRACT_MODE_SET: {
      const { extractMode } = action;
      // v0.6.4 T19-1：切 Tab 清空对方输入与结果（符合 docs/01-页面与交互说明.md:292
      // 「切换 Radio 时清空当前输入与结果」隔离要求）。规则选择与 tag 过滤保留。
      return {
        ...state,
        extractMode,
        extractInput: "",
        extractResult: null,
      };
    }
    case ACTION.EXTRACT_INPUT_SET: {
      const { extractInput } = action;
      return { ...state, extractInput };
    }
    case ACTION.EXTRACT_RESULT_SET: {
      const { extractResult } = action;
      return { ...state, extractResult };
    }
    case ACTION.EXTRACT_LOADING_SET: {
      const { extractLoading } = action;
      return { ...state, extractLoading };
    }
    // v0.4.4 T10-3：extract 规则选择状态。
    case ACTION.EXTRACT_SELECTED_INDICES_SET: {
      const { extractSelectedIndices } = action;
      return { ...state, extractSelectedIndices };
    }
    case ACTION.EXTRACT_RULE_TAG_FILTER_SET: {
      const { extractRuleTagFilter } = action;
      return { ...state, extractRuleTagFilter };
    }
    default:
      return state;
  }
};

// 顶层 UI：actionHint 操作提示 + loading 全局加载标志 + RESET 全量重置。
const uiDomain = (state, action) => {
  switch (action.type) {
    case ACTION.HINT_SET: {
      const { actionHint } = action;
      return { ...state, actionHint };
    }
    case ACTION.LOADING_SET: {
      const { loading } = action;
      return { ...state, loading };
    }
    case ACTION.RESET:
      return { ...initialState };
    default:
      return state;
  }
};

// 主 reducer：顺序分派到各领域函数。每个领域函数对不相关 action 返回 state
// 原样，故未匹配的 action 最终原样返回 state。SET_FILE 的级联清空由 fileDomain
// 集中处理（其他领域函数对 SET_FILE 返回原样，不清空各自字段）。
export function appReducer(state, action) {
  state = fileDomain(state, action);
  state = maskDomain(state, action);
  state = validateDomain(state, action);
  state = logDomain(state, action);
  state = pcapDomain(state, action);
  state = rulesDomain(state, action);
  state = exportDomain(state, action);
  state = navDomain(state, action);
  state = toolsDomain(state, action);
  state = searchDomain(state, action);
  state = settingsDomain(state, action);
  state = extractDomain(state, action);
  state = uiDomain(state, action);
  return state;
}
