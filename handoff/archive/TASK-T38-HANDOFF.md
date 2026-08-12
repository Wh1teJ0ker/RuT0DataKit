```yaml
task_id: T38
goal: |
  在前端接入 base64_column IPC 封装 + 在 ColumnOpsPanel 新增「Base64 编解码」表单：
  选列 + 选模式（编码/解码）→ 调 base64_column → 刷新当前页 + 刷新 undoStack。
in_scope:
  - frontend/src/tauri.js（新增 base64Column IPC 封装）
  - frontend/src/components/panels/ColumnOpsPanel.jsx（新增 Base64 编解码 Form）
out_of_scope:
  - 不改后端（T37 已做）
  - 不改 state/reducer（不需要新 ACTION，复用 SET_SHEET_DATA + SET_UNDO_STACK）
  - 不改其他面板
acceptance_criteria:
  - ColumnOpsPanel 有「Base64 编解码 — 目标列」Select + 「模式」Select（编码/解码）+ 按钮
  - 点击「Base64 编码」→ 列值变 Base64；message.success 提示 affected/skipped
  - 点击「Base64 解码」→ 列值恢复原文
  - 操作后刷新当前页数据（getSheetData + SET_SHEET_DATA）
  - 操作后刷新 undoStack（listUndoableOperations + SET_UNDO_STACK）
  - pnpm build 通过
verification_commands:
  - pnpm --prefix frontend build
files_likely_to_change:
  - frontend/src/tauri.js
  - frontend/src/components/panels/ColumnOpsPanel.jsx
risks:
  - 无 sheet 时按钮应禁用或提示
  - 解码失败行不影响成功的行
depends_on: [T37]
status: planned
```

## 实现指引

### tauri.js IPC 封装

```javascript
export async function base64Column(sheetId, column, mode) {
  return invoke("base64_column", { sheetId, column, mode });
}
```

`mode` 传字符串 `"encode"` / `"decode"`（后端 Base64Mode serde lowercase）。

### ColumnOpsPanel.jsx

在现有 JSON 解析 Form 下方新增第二个 Form（独立 Form 实例）：

```jsx
const [base64Form] = Form.useForm();
const [base64ing, setBase64ing] = useState(false);

async function handleBase64() {
  if (!sheet) { message.warning("请先导入数据"); return; }
  const column = base64Form.getFieldValue("column");
  const mode = base64Form.getFieldValue("mode");
  if (!column) { message.warning("请选择目标列"); return; }
  if (!mode) { message.warning("请选择模式"); return; }
  setBase64ing(true);
  try {
    const res = await base64Column(sheet.id, column, mode);
    // 刷新当前页
    const data = await getSheetData(sheet.id, sheet.page || 1, sheet.pageSize || PAGE_SIZE);
    dispatch({ type: "SET_SHEET_DATA", payload: { ...data, sheetId: sheet.id } });
    // 刷新 undoStack
    const ops = await listUndoableOperations(sheet.id);
    dispatch({ type: "SET_UNDO_STACK", payload: ops });
    message.success(`Base64 ${mode === "encode" ? "编码" : "解码"}完成：${res.affected} 行，跳过 ${res.skipped ?? 0}`);
  } catch (e) {
    console.error("base64_column failed:", e);
    message.error(`操作失败：${e}`);
  } finally {
    setBase64ing(false);
  }
}
```

UI 结构：

```jsx
<Divider style={{ margin: "8px 0" }} />
<Form form={base64Form} layout="vertical" size="small">
  <Form.Item label="Base64 编解码 — 目标列" name="column">
    <Select placeholder="选择列" options={headers.map(h => ({ label: h, value: h }))} showSearch optionFilterProp="label" />
  </Form.Item>
  <Form.Item label="模式" name="mode" initialValue="encode">
    <Select options={[{ label: "编码", value: "encode" }, { label: "解码", value: "decode" }]} />
  </Form.Item>
  <Form.Item>
    <Button type="primary" loading={base64ing} onClick={handleBase64} block>
      执行 Base64
    </Button>
  </Form.Item>
</Form>
```

需要从 antd 额外 import `Divider`，从 tauri.js import `base64Column` + `listUndoableOperations`。
