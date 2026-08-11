# TASK-T74-HANDOFF

```yaml
task_id: T74
goal: |
  扩展前端 CryptoPanel.jsx：在既有 Base64 编解码 UI 基础上，增加哈希算法选择
  （MD5/SHA1/SHA256），新增 `hashColumn` IPC wrapper，执行后刷新当前页数据 + undo 栈。
  操作模式从「单选 Base64 编/解码」扩展为「操作类型 + 算法」二级选择。

in_scope:
  - frontend/src/components/panels/CryptoPanel.jsx（扩展 UI + handler）
  - frontend/src/tauri.js（新增 hashColumn wrapper）

out_of_scope:
  - 不改后端（T73 负责）
  - 不改 SidePanel.jsx / TopToolbar.jsx（crypto capability 已注册，无需改）
  - 不改 reducer.js / state/factory.js（复用既有 SET_SHEET_DATA + SET_UNDO_STACK）
  - 不改 capabilities/default.json
  - 不创建 tag / 不合并 main

acceptance_criteria:
  - CryptoPanel 显示操作类型选择（Base64 编/解码 / MD5 / SHA1 / SHA256）
  - 选 Base64 时显示 mode（encode/decode）子选项；选哈希时不显示 mode（哈希不可逆）
  - 选哈希算法后点「执行」按钮，调用 hashColumn(sheetId, column, algorithm)
  - 执行成功后刷新当前页数据（getSheetData + SET_SHEET_DATA）
  - 执行成功后刷新 undo 栈（listUndoableOperations + SET_UNDO_STACK）
  - message.success 显示 affected/skipped 数量
  - hashColumn wrapper 在 tauri.js 导出，invoke "hash_column"，参数 { sheetId, column, algorithm }
  - pnpm --prefix frontend build exit 0

verification_commands:
  - pnpm --prefix frontend build

files_likely_to_change:
  - frontend/src/components/panels/CryptoPanel.jsx
  - frontend/src/tauri.js

risks:
  - antd Form 动态显示/隐藏 mode 子选项需要 shouldUpdate 或 condition render
  - algorithm 字符串必须与后端 serde lowercase 对齐（"md5"/"sha1"/"sha256"）
  - 哈希不可逆，UI 应提示用户此操作不可解码还原（但可撤销）

depends_on: [T73]
status: planned
```

## 实现指引

### CryptoPanel.jsx UI 设计

当前 CryptoPanel 结构（简化）：
```jsx
<Form form={form}>
  <Form.Item label="目标列" name="column"><Select options={columns} /></Form.Item>
  <Form.Item label="模式" name="mode"><Select options={[{value:"encode"},{value:"decode"}]} /></Form.Item>
  <Button onClick={handleBase64}>执行 Base64</Button>
</Form>
```

扩展后：
```jsx
<Form form={form}>
  <Form.Item label="目标列" name="column"><Select options={columns} /></Form.Item>
  <Form.Item label="操作" name="op">
    <Select options={[
      { value: "base64_encode", label: "Base64 编码" },
      { value: "base64_decode", label: "Base64 解码" },
      { value: "md5", label: "MD5 哈希" },
      { value: "sha1", label: "SHA1 哈希" },
      { value: "sha256", label: "SHA256 哈希" },
    ]} />
  </Form.Item>
  <Button onClick={handleExecute}>执行</Button>
</Form>
```

`handleExecute`：
- 取 `op` 值
- `op` 以 `base64_` 开头 → 调 `base64Column(sheetId, column, op === "base64_encode" ? "encode" : "decode")`
- `op` 是 md5/sha1/sha256 → 调 `hashColumn(sheetId, column, op)`
- 成功后：`getSheetData` + `SET_SHEET_DATA`；`listUndoableOperations` + `SET_UNDO_STACK`；`message.success`

### tauri.js wrapper

```js
export function hashColumn(sheetId, column, algorithm) {
  return invoke("hash_column", { sheetId, column, algorithm });
}
```

algorithm 传 "md5" / "sha1" / "sha256"（与后端 HashAlgorithm serde lowercase 对齐）。

### 参考既有 base64Column wrapper

`frontend/src/tauri.js:412`：
```js
export function base64Column(sheetId, column, mode) {
  return invoke("base64_column", { sheetId, column, mode });
}
```

### 注意

- CryptoPanel.jsx 既有 header docstring（L14-17）已提及"后续将扩展 URL-safe Base64 / MD5 / SHA / AES"，本次实现 MD5/SHA1/SHA256，更新 docstring
- 哈希操作不可逆但可撤销（before_snapshot 回写），UI 可加一行 hint 文案"哈希不可逆，可通过撤销恢复原文"
- 不要在 CryptoPanel 引入新依赖（antd Form/Select/Button/Input 已有）
