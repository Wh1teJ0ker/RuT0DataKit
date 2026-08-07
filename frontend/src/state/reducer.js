// reducer + patchActiveSheet（T13 拆分自原 state.js monolith）。
// 分支逻辑、action 语义、Sheet 工厂返回结构全部保持不变。
// 注：initialState 仅由 AppContext.js 用于 useReducer 初始化；这里不引入
// 以免无引用 import 报 ESLint no-unused-vars。本文件只导出 reducer / patchActiveSheet。
import { ACTION } from "./constants";
import { createEmptySheet, createSheetFromImport, defaultSheetName } from "./factory";
import { toRowObjects } from "../tauri";

export function patchActiveSheet(state, patch) {
  if (!state.activeSheetId) return state;
  return {
    ...state,
    sheets: state.sheets.map((s) =>
      s.id === state.activeSheetId ? { ...s, ...patch(s) } : s
    ),
  };
}

export function reducer(state, action) {
  switch (action.type) {
    // ---- T2 ----
    case ACTION.SET_VIEW:
      return { ...state, currentView: action.payload };
    case ACTION.SET_ACTIVE_CAPABILITY: {
      const next =
        action.payload === state.activeCapability ? null : action.payload;
      return { ...state, activeCapability: next };
    }
    case ACTION.SET_AI_PANEL_VISIBLE:
      return {
        ...state,
        aiPanel: { ...state.aiPanel, visible: Boolean(action.payload) },
      };

    // ---- T3 ----
    case ACTION.ADD_SHEET: {
      // 新建产生空表（无列、无行）。payload = { sheet } | { name } | {}
      const sheet =
        action.payload?.sheet ||
        createEmptySheet(action.payload?.name || defaultSheetName());
      return {
        ...state,
        sheets: [...state.sheets, sheet],
        activeSheetId: sheet.id,
      };
    }
    case ACTION.CLOSE_SHEET: {
      const id = action.payload;
      const idx = state.sheets.findIndex((s) => s.id === id);
      if (idx < 0) return state;
      const nextSheets = state.sheets.filter((s) => s.id !== id);
      let nextActive = state.activeSheetId;
      if (state.activeSheetId === id) {
        // 关闭后激活相邻 Tab：优先右邻，否则左邻。
        nextActive = nextSheets.length
          ? nextSheets[Math.min(idx, nextSheets.length - 1)].id
          : null;
      }
      return { ...state, sheets: nextSheets, activeSheetId: nextActive };
    }
    case ACTION.SET_ACTIVE_SHEET:
      return { ...state, activeSheetId: action.payload };
    case ACTION.RENAME_SHEET:
      return {
        ...state,
        sheets: state.sheets.map((s) =>
          s.id === action.payload.id ? { ...s, name: action.payload.name } : s
        ),
      };
    case ACTION.SET_SELECTION:
      // payload = { selectedRowKeys, lastSelectedIndex }
      return patchActiveSheet(state, () => ({
        selection: {
          selectedRowKeys: action.payload.selectedRowKeys,
          lastSelectedIndex:
            action.payload.lastSelectedIndex !== undefined
              ? action.payload.lastSelectedIndex
              : null,
        },
      }));
    case ACTION.REORDER_COLUMNS:
      // payload = columnOrder (string[])
      return patchActiveSheet(state, () => ({
        columnOrder: action.payload,
      }));
    case ACTION.SET_COLUMN_VISIBILITY:
      // payload = { [header]: boolean } | columnVisibility 全量对象
      return patchActiveSheet(state, (s) => ({
        columnVisibility: { ...s.columnVisibility, ...action.payload },
      }));
    case ACTION.SET_PAGE:
      // payload = page
      return patchActiveSheet(state, () => ({ page: action.payload }));

    // ---- T5（导入流）----
    case ACTION.IMPORT_SUCCESS: {
      // payload = ImportResult { sessionId, sheetId, rowCount, headers } + name
      const sheet = createSheetFromImport(action.payload);
      // 若已存在同 sheetId 的 Sheet，替换之；否则追加。
      const exists = state.sheets.some((s) => s.id === sheet.id);
      const sheets = exists
        ? state.sheets.map((s) => (s.id === sheet.id ? sheet : s))
        : [...state.sheets, sheet];
      return { ...state, sheets, activeSheetId: sheet.id };
    }
    case ACTION.SET_SHEET_DATA:
      // payload = { sheetId, headers, rows, total, page, pageSize }
      return {
        ...state,
        sheets: state.sheets.map((s) => {
          if (s.id !== action.payload.sheetId) return s;
          // PageData.rows 为 Vec<Vec<Option<String>>>；复用 tauri.js 的
          // toRowObjects 纯函数转成 antd 行对象（与导出共用，避免重复实现）。
          const headers = action.payload.headers || s.headers;
          const rows = toRowObjects(
            action.payload.rows,
            headers,
            action.payload.sheetId,
            action.payload.page
          );
          return {
            ...s,
            headers,
            rows,
            total: action.payload.total ?? s.total,
            page: action.payload.page ?? s.page,
            pageSize: action.payload.pageSize ?? s.pageSize,
            columnOrder: [...headers],
            columnVisibility: headers.reduce(
              (acc, h) => ({ ...acc, [h]: s.columnVisibility?.[h] !== false }),
              {}
            ),
          };
        }),
      };
    // ---- tshark 设置（v1.0.0 全格式扩展）----
    case ACTION.SET_TSHARK_PATH:
      return { ...state, tsharkPath: action.payload || null };
    case ACTION.SET_TSHARK_DETECTED:
      return { ...state, tsharkDetected: action.payload || null };
    case ACTION.SET_TSHARK_LOADING:
      return { ...state, tsharkLoading: Boolean(action.payload) };

    // ---- v1.1.0 行状态高亮（脱敏/校验/提取）----
    case ACTION.APPLY_ROW_STATUSES: {
      // payload = { sheetId, rowStatuses: { [rowKey]: "default"|"invalid"|"masked"|"hit" } }
      const { sheetId, rowStatuses } = action.payload;
      return {
        ...state,
        sheets: state.sheets.map((s) => {
          if (s.id !== sheetId) return s;
          const rows = s.rows.map((r) =>
            rowStatuses[r.key] ? { ...r, status: rowStatuses[r.key] } : r
          );
          return {
            ...s,
            rows,
            statusHighlights: { ...s.statusHighlights, ...rowStatuses },
          };
        }),
      };
    }

    default:
      return state;
  }
}

