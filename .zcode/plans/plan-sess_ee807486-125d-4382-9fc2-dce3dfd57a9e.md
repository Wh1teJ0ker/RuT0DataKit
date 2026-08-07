# 导出面板重构：下拉格式选择 + 每格式独立选项 + TXT 格式设置 + 修复全表导出 BUG

## 一、BUG 根因（修复目标）

当前导出（CSV/JSON/TXT）只读取 `sheet.rows`，而 `sheet.rows` 仅含**当前页**（≤50 行，`PAGE_SIZE`）。数据 >50 行或翻页后导出 → 缺失部分表现为"空白"。

**修复**：导出前先拉全表数据。后端 `get_sheet_data` 的 SQL `LIMIT ?page_size OFFSET ?offset` 接受任意 `page_size`，传 `page_size = sheet.total`（含表头）即可一次取全所有数据行（后端在 `data.rs:201-203` 跳过 row_idx=0 表头行）。**无需新增 IPC 或改 DB schema**。

---

## 二、UI 重构：ExportModal.jsx

### 1. 格式选择器：Radio.Group → `Select`（下拉列表）
- 用 antd `Select`（项目中首次引入，但 `Dropdown` 在 DataTable.jsx:178 已有先例，风格一致）
- 4 项：CSV / JSON / TXT / XLSX(开发中,disabled)
- 选中后下方动态渲染**该格式专属选项面板**（用条件渲染 + `Divider` 分隔，不用 Collapse 保持简洁）

### 2. 每格式独立选项

**TXT（核心，按用户要求"可设置格式"）**：
- 模板字符串 `Input.TextArea`：`{字段名}` 引用字段值，其它字符字面输出；**留空 → 用下面的分隔符模式**
- 列分隔符 `Select`：Tab / 逗号 `,` / 分号 `;` / 竖线 `|` / 自定义（选中自定义时出 `Input`）
- 行尾 `Select`：CRLF (Windows) / LF (Unix)
- 含表头行 `Switch`：是否在首行写字段名（模板模式下默认关，分隔符模式下默认开）
- 列选择 Checkbox（保留现有，仅 TXT 时显示，控制模板/分隔符作用列）

**CSV**：
- 列分隔符 `Select`：逗号(默认) / 分号 / Tab / 自定义
- 含表头行 `Switch`（默认开）
- 列选择 Checkbox（保留）

**JSON**：
- 缩进 `Select`：2 空格(默认) / 4 空格 / 紧凑(单行)
- 格式 `Select`：数组 `[{},{}]`(默认) / NDJSON(每行一对象)
- 列选择 Checkbox（保留）

**XLSX**：disabled，显示「开发中」

### 3. 布局
```
Modal
├─ "选择导出格式：" + Select (下拉)
├─ Divider
├─ {format === 'txt' && <TxtOptions/>}   ← 模板/分隔符/行尾/表头
├─ {format === 'csv' && <CsvOptions/>}   ← 分隔符/表头
├─ {format === 'json' && <JsonOptions/>} ← 缩进/格式
├─ Divider
└─ <ColumnPicker/> (通用，所有格式都显示，TXT 也保留)
```
合并现有「TXT 时显示列选择」+「非 TXT 显示列选择」两个重复块为单一 `<ColumnPicker>`。

---

## 三、tauri.js 重构

### 1. 新增 `fetchAllRowsForExport(sheet)`
```js
// 一次拉全表（page=1, pageSize=total），返回 { headers, rows: [...] }
// rows 形态与 SET_SHEET_DATA 一致（antd 行对象：{key, [header]: value|null}）
async function fetchAllRowsForExport(sheet) {
  const total = sheet.total ?? sheet.rows.length;
  const data = await getSheetData(sheet.id, 1, total);
  // 复用 reducer 的行对象构造逻辑（抽成纯函数 toRowObjects）
  return { headers: data.headers, rows: toRowObjects(data.rows, data.headers, sheet.id) };
}
```
抽 `toRowObjects(rawRows, headers, sheetId)` 纯函数（从 reducer.js:113-120 提取），reducer 和 export 共用，避免重复。

### 2. 导出函数接收 options + 内部拉全表

```js
exportSheetToCsv(sheet, { separator, withHeader, headers: selectedCols })
exportSheetToJson(sheet, { indent, ndjson, headers: selectedCols })
exportSheetToTxt(sheet, { template, separator, lineEnding, withHeader, headers: selectedCols })
```
每个函数内部：`const { headers, rows } = await fetchAllRowsForExport(sheet)` → 按 `selectedCols` 过滤 → 按 options 渲染 → `saveTextFile`。

### 3. CSV esc 支持自定义分隔符
现有 `esc` 检测 `/[",\n\r]/`，改为检测 `/"|\n|\r|<sep>`（含分隔符字符时也要加引号）。

---

## 四、改动文件

| 文件 | 改动 |
|------|------|
| `frontend/src/components/ExportModal.jsx` | 重写：Select 格式 + 条件选项面板 + 单一 ColumnPicker；handleOk 传 options |
| `frontend/src/tauri.js` | +`fetchAllRowsForExport`、+`toRowObjects`、3 个导出函数签名加 options + 内部拉全表、CSV esc 适配分隔符 |
| `frontend/src/state/reducer.js` | SET_SHEET_DATA 改用抽出的 `toRowObjects`（纯函数，行为不变） |

**不改**：后端 Rust（无新 IPC）、constants.js、App.jsx（导出流不经过 App）、TopToolbar.jsx（只开弹窗）。

---

## 五、验证

`cargo tauri dev` → 导入一个 100+ 行的表：
1. 导出 CSV → 文件含全部 100+ 行（修复前只 50 行）✅
2. 导出 TXT 模板 `{content}` → 每行一条记录，全量 ✅
3. 导出 TXT 留空模板 + 分隔符选竖线 + 含表头 → `h1|h2\nv1|v2` 全量 ✅
4. 导出 JSON → `[{},{}]` 全量，NDJSON → 每行一对象 ✅
5. 导出后文件无空白行/缺失行 ✅
6. 翻到第 2 页再导出 → 仍是全表（不依赖当前页）✅
7. UI：下拉选格式 → 下方动态出对应选项面板，切换格式选项跟随切换 ✅

## 六、不做
- 不加 XLSX 实现（保留开发中占位）
- 不新增后端 IPC
- 不提交 git（等用户验收）