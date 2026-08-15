// 共享工具：DataTable 单元格搜索命中高亮渲染。

import { CELL_SIZE_LIMIT } from "../constants";

// 模块级 TextEncoder / TextDecoder，避免每次 highlightCell 调用重新分配。
const _encoder = new TextEncoder();
const _decoder = new TextDecoder("utf-8", { fatal: false });

/**
 * 把单元格值按命中区间（字节偏移）用 `<mark>` 包裹渲染。
 *
 * 后端 start/end 为 Rust str::find / regex::find 返回的 **字节偏移**，
 * 不能直接用 JS string slice（UTF-16 code unit 索引）——中文等多字节字符会导致
 * end > text.length 误判越界。这里用 TextEncoder 取 UTF-8 字节、TextDecoder
 * 按字节区间还原字符串，保证偏移语义一致。
 *
 * 大单元格防护：超过 CELL_SIZE_LIMIT 的文本跳过 TextEncoder.encode，直接返回纯文本，
 * 防止 8.5MB 级大单元格在主线程上分配巨型 Uint8Array 冻结 UI。
 *
 * @param {string|null} value       单元格原始值
 * @param {Array<[number, number]>} hitRanges  命中区间数组 [[start, end], ...]（字节偏移）
 * @returns {string|JSX.Element}  纯文本或 <span>…<mark>…</mark>…</span>
 */
export function highlightCell(value, hitRanges) {
  if (value == null) return value;
  const text = String(value);
  if (!hitRanges || hitRanges.length === 0) return text;
  // 大单元格防护：跳过 TextEncoder.encode，直接返回纯文本（无 mark 高亮）
  if (text.length > CELL_SIZE_LIMIT) return text;
  const bytes = _encoder.encode(text);
  const byteLen = bytes.length;
  const sorted = [...hitRanges].sort((a, b) => a[0] - b[0]);
  const parts = [];
  let cursor = 0; // 字节游标
  for (const [start, end] of sorted) {
    if (start < cursor) continue; // 越界/重叠，跳过
    if (start > byteLen || end > byteLen) break; // 超出字节长度，终止
    if (start > cursor) {
      parts.push(_decoder.decode(bytes.subarray(cursor, start)));
    }
    parts.push(
      <mark key={`${start}-${end}`} style={{ background: "#fff48f" }}>
        {_decoder.decode(bytes.subarray(start, end))}
      </mark>
    );
    cursor = end;
  }
  if (cursor < byteLen) parts.push(_decoder.decode(bytes.subarray(cursor)));
  return <span>{parts}</span>;
}
