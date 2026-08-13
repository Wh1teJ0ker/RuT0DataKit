// reducer + patchActiveSheet（T13 拆分自原 state.js monolith）。
// 分支逻辑、action 语义、Sheet 工厂返回结构全部保持不变。
// 注：initialState 仅由 AppContext.js 用于 useReducer 初始化；这里不引入
// 以免无引用 import 报 ESLint no-unused-vars。本文件只导出 reducer / patchActiveSheet。
import { ACTION } from "./constants";
import {
  createEmptySheet,
  createSheetFromImport,
  createSheetFromParse,
  defaultSheetName,
} from "./factory";
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
    case ACTION.SET_AI_PANEL_PINNED:
      return {
        ...state,
        aiPanel: { ...state.aiPanel, pinned: Boolean(action.payload) },
      };
    case ACTION.SET_SIDE_PANEL_COLLAPSED:
      return {
        ...state,
        sidePanel: { ...state.sidePanel, collapsed: Boolean(action.payload) },
      };
    case ACTION.SET_SIDE_PANEL_PINNED:
      return {
        ...state,
        sidePanel: { ...state.sidePanel, pinned: Boolean(action.payload) },
      };

    // ---- T3 ----
    case ACTION.ADD_SHEET: {
      // 新建产生空表（无列、无行）。payload = { sheet } | { name } | {}
      // v1.1.2：pageSize 继承全局 state.pageSize。
      const sheet =
        action.payload?.sheet ||
        createEmptySheet(action.payload?.name || defaultSheetName(), state.pageSize);
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
      // v1.1.2：pageSize 继承全局 state.pageSize。
      const sheet = createSheetFromImport(action.payload, state.pageSize);
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
            action.payload.page,
            action.payload.pageSize ?? s.pageSize
          );
          // T63：回填行状态高亮——翻页后从 statusHighlights map 恢复
          // 脱敏/校验/提取标记。toRowObjects 默认 status="default"，这里按
          // row.key 查 statusHighlights 覆盖。statusHighlights 由
          // APPLY_ROW_STATUSES 写入，翻页不丢失（不在本 action 中清空）。
          const rowsWithStatus = rows.map((r) => {
            const savedStatus = s.statusHighlights?.[r.key];
            return savedStatus ? { ...r, status: savedStatus } : r;
          });
          // T63：columnOrder 保留——用户拖拽重排列后翻页不丢失。
          // 仅当 s.columnOrder 已存在且与 headers 集合一致时保留旧顺序；
          // 否则（首次加载 / headers 变化）用 [...headers]。
          const orderUnchanged =
            s.columnOrder &&
            s.columnOrder.length === headers.length &&
            s.columnOrder.every((h) => headers.includes(h)) &&
            headers.every((h) => s.columnOrder.includes(h));
          const columnOrder = orderUnchanged ? s.columnOrder : [...headers];
          return {
            ...s,
            headers,
            rows: rowsWithStatus,
            total: action.payload.total ?? s.total,
            page: action.payload.page ?? s.page,
            pageSize: action.payload.pageSize ?? s.pageSize,
            columnOrder,
            // 单遍构造（Object.fromEntries），避免 reduce + spread 的 O(C²) 复制；
            // 保留 s.columnVisibility 既有值，未配置默认 true（与旧 reduce 语义一致）。
            columnVisibility: Object.fromEntries(
              headers.map((h) => [h, s.columnVisibility?.[h] !== false])
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

    // ---- v1.1.2 全局每页行数 ----
    case ACTION.SET_PAGE_SIZE: {
      // payload = number
      // 同步全局 + 所有 Sheet 的 pageSize（page 重置为 1 防越界）。
      // 当前激活 Sheet 的 rows 由 PageSizeCard 再调 getSheetData 刷新。
      const pageSize = action.payload;
      return {
        ...state,
        pageSize,
        sheets: state.sheets.map((s) => ({ ...s, pageSize, page: 1 })),
      };
    }

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

    // ---- v1.1.1 撤销 / 搜索 / 列操作 ----
    case ACTION.SET_SEARCH_STATE:
      // payload = { query?, useRegex?, colIdx?, page? }（部分更新）
      return {
        ...state,
        searchState: { ...state.searchState, ...action.payload },
      };
    case ACTION.APPLY_SEARCH_HITS: {
      // payload = { sheetId, hits: Array<{ rowIdx, colIdx, value, matches: Array<{start, end}> }> }
      // 把后端 rowIdx（DB 绝对行号，row_idx=0 是表头行，数据从 1 开始）转成当前页行 key：
      //   pageBase = (sheet.page - 1) * sheet.pageSize
      //   pageInnerIdx = rowIdx - 1 - pageBase（减 1 跳过表头行）
      //   rowKey = `${sheetId}-${sheet.page}-${pageInnerIdx}`
      // 越界（不在当前页 / 列号超长）跳过该 hit。
      //
      // v1.1.1 hotfix：每次先清空 searchHits（不再 spread 旧值），避免「第一次搜索的高亮
      // 在第二次搜索后仍显示」的 stale highlight bug。
      const { sheetId, hits } = action.payload;
      return {
        ...state,
        sheets: state.sheets.map((s) => {
          if (s.id !== sheetId) return s;
          const pageBase = (s.page - 1) * s.pageSize;
          const pageLen = Array.isArray(s.rows) ? s.rows.length : 0;
          const searchHits = {}; // 先清空，再写入本次命中
          for (const hit of hits || []) {
            const pageInnerIdx = hit.rowIdx - 1 - pageBase;
            if (pageInnerIdx < 0 || pageInnerIdx >= pageLen) continue; // 不在当前页
            const colHeader = s.headers[hit.colIdx];
            if (colHeader === undefined) continue; // 列号越界
            const rowKey = `${sheetId}-${s.page}-${pageInnerIdx}`;
            if (!searchHits[rowKey]) searchHits[rowKey] = {};
            searchHits[rowKey][colHeader] = (hit.matches || []).map((m) => [
              m.start,
              m.end,
            ]);
          }
          return { ...s, searchHits };
        }),
      };
    }
    case ACTION.APPLY_SEARCH_ROWS: {
      // v1.1.1 hotfix 行级搜索：payload = { sheetId, rows: SearchRowsPage }
      //   rows: { rows: Array<{ rowIdx, cells: Array<Option<String>>, hits: Array<{colIdx, value, matches}> }>,
      //           total, page, pageSize }
      // 把后端行级命中转成 antd 行对象（toRowObjects），写入 sheet.searchRows / searchTotal，
      // 同时同步一份 searchHits（按行 key 即 `${sheetId}-${page}-${i}`，便于 DataTable
      // 复用 highlightCell）。searchRows !== null 即表示「搜索态」。
      const { sheetId, rows: page } = action.payload;
      return {
        ...state,
        sheets: state.sheets.map((s) => {
          if (s.id !== sheetId) return s;
          const list = page?.rows || [];
          const total = page?.total ?? 0;
          const pageNum = page?.page ?? 1;
          // 转 antd 行对象：key = `${sheetId}-${pageNum}-${i}`，i 为页内 0-based 下标。
          const searchRows = list.map((r, i) => {
            const obj = { key: `${sheetId}-${pageNum}-${i}` };
            s.headers.forEach((h, ci) => {
              obj[h] = r.cells?.[ci] ?? null;
            });
            return obj;
          });
          // 同步 searchHits：行 key → colHeader → [[start, end], ...]
          const searchHits = {};
          list.forEach((r, i) => {
            const rowKey = `${sheetId}-${pageNum}-${i}`;
            const rowHits = {};
            for (const h of r.hits || []) {
              const colHeader = s.headers[h.colIdx];
              if (colHeader === undefined) continue;
              rowHits[colHeader] = (h.matches || []).map((m) => [m.start, m.end]);
            }
            if (Object.keys(rowHits).length > 0) searchHits[rowKey] = rowHits;
          });
          return {
            ...s,
            searchRows,
            searchTotal: total,
            searchHits,
          };
        }),
      };
    }
    case ACTION.CLEAR_SEARCH: {
      // 清空当前 Sheet 高亮 + searchRows + 重置顶层搜索状态。
      const cleared = patchActiveSheet(state, (s) => ({
        searchHits: {},
        searchRows: null,
        searchTotal: 0,
      }));
      return {
        ...cleared,
        searchState: { query: "", useRegex: false, colIdx: null, page: 1 },
      };
    }
    case ACTION.ADD_SHEET_FROM_PARSE: {
      // payload = ParseResult（含 newSheetId/headers/rowCount/skipped + sessionId/name?/column?）
      // 新 Sheet 必然新 id，不替换同名；追加并激活。
      // v1.1.2：pageSize 继承全局 state.pageSize。
      const sheet = createSheetFromParse(action.payload, state.pageSize);
      return {
        ...state,
        sheets: [...state.sheets, sheet],
        activeSheetId: sheet.id,
      };
    }
    case ACTION.SET_UNDO_STACK:
      // payload = Array<{ id, kind, createdAt }>
      return { ...state, undoStack: action.payload };

    default:
      return state;
  }
}

