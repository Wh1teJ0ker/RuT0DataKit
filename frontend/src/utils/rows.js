// 共享工具：后端 PageData.rows → antd 行对象转换。

import { PAGE_SIZE } from "../constants";

/**
 * 把后端 `Vec<Vec<Option<String>>>` 原始行转为 antd Table 行对象。
 *
 * 行 key = `${sheetId}-${page}-${i}`（i 为页内 0-based 下标），稳定唯一。
 * `_rowIdx` 为全局行号（1-based），供导出/序号列使用。
 * `status` 默认 "default"，由 APPLY_ROW_STATUSES 覆盖。
 *
 * 从 tauri/export.js 移入 utils/rows.js，reducer 与 export 共用同一实现，
 * 避免 APPLY_SEARCH_ROWS 分支内联重建行对象（遗漏 _rowIdx / status 字段）。
 *
 * @param {Array<Array<string|null>>} rawRows  后端 PageData.rows
 * @param {string[]} headers  字段名顺序
 * @param {number} sheetId  用于生成稳定 key
 * @param {number} [page=1]  当前页码（仅用于 key 区分）
 * @param {number} [pageSize=PAGE_SIZE]  每页行数（用于 _rowIdx 全局行号计算）
 * @returns {Array<object>} antd 行对象数组
 */
export function toRowObjects(rawRows, headers, sheetId, page = 1, pageSize = PAGE_SIZE) {
  const base = (page - 1) * pageSize;
  return rawRows.map((row, i) => {
    const obj = { key: `${sheetId}-${page}-${i}`, _rowIdx: base + i + 1 };
    headers.forEach((h, col) => {
      obj[h] = row[col] ?? null;
    });
    obj.status = "default";
    return obj;
  });
}
