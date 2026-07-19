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
  // 规则（全局规则库）：RulesView 管理，独立于文件导入。
  // MaskView/ValidateView 不直接写入这里；脱敏/校验应用时用
  // maskOverrides/validateOverrides 优先 + 全局 rules 兜底合并。
  rules: { maskers: [], validators: [] }, // RuleSet 形状
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
  exportFormat: "csv", // "csv" | "xlsx"
  exportSource: "masked", // ExportView 用：导出源 "masked" | "validate" | "raw"
  validateFilter: "all", // ExportView 用：校验后行过滤 "all" | "valid" | "invalid"
  // 导航
  activeView: "mask",
  // UI
  actionHint: "",
  loading: false,
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
