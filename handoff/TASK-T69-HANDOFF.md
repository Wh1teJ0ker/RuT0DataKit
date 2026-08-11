# TASK-T69-HANDOFF

```yaml
task_id: T69
goal: |
  重做前端校验页面为统一表单（非两个 Tab）：用户可添加多条「列 + 校验规则」组合，
  身份证规则行可勾选「对比性别一致性」+「对比出生日期一致性」并指定对应列，
  一个「校验」按钮 → 调 validateMultiRulesToTwoSheets → 双 Tab 落地。
  新增 tauri.js wrapper，注册命令无需改 capabilities。重写 docs/versions/1.1.4/ 文档。

in_scope:
  - frontend/src/components/panels/ValidatePanel.jsx  # 重做为统一校验表单
  - frontend/src/tauri.js                              # 新增 validateMultiRulesToTwoSheets wrapper
  - docs/versions/1.1.4/更新日志.md                     # 重写以反映重设计
  - docs/versions/1.1.4/RELEASE-NOTES.md               # 重写

out_of_scope:
  - 不改后端（T67/T68 已完成）
  - 不改 frontend/src/components/layout/TopToolbar.jsx（已移除 rowValidate）
  - 不改 frontend/src/components/layout/SidePanel.jsx（已移除 rowValidate）
  - 不改 frontend/src/state/*（复用 ADD_SHEET_FROM_PARSE + SET_SHEET_DATA）
  - 不改 frontend/src/App.jsx（validate 走默认 SidePanel 分支）
  - 不改 src-tauri/capabilities/default.json（自定义命令无需权限）

acceptance_criteria:
  - ValidatePanel.jsx 是单一表单（无 Tabs/Segmented 切换），顶部可动态添加多条规则行
  - 每条规则行：目标列 Select + 校验规则 Select（options 来自 listRules 的 kind=validate 规则）+ 删除按钮
  - 校验规则 options 包含 T67 新增的 7 条 validate 规则 + 现有 name-validate（共 7 条函数式 + 正则）
  - 当选中的规则是 idcard-validate 时，该行展开跨字段配置：勾选「对比性别一致性」+ Select 性别列；勾选「对比出生日期一致性」+ Select 出生日期列
  - 底部有手机号前缀白名单 Select mode="tags"（可选，应用于 phone-validate 规则）
  - 一个「校验」按钮 → 调 validateMultiRulesToTwoSheets(sheetId, sessionId, rules, phonePrefixes)
  - 双 Tab 落地：addSheetFromParse + getSheetData + SET_SHEET_DATA（复用现有 landSheet 模式）
  - 汇总消息：通过/失败行数 + top 失败原因（复用 RowValidatePanel 的 tally 逻辑）
  - tauri.js 新增 validateMultiRulesToTwoSheets wrapper
  - docs/versions/1.1.4/更新日志.md 重写：记录 T67/T68/T69 三个任务 + 统一校验页面设计
  - docs/versions/1.1.4/RELEASE-NOTES.md 重写：聚焦统一校验页面
  - pnpm --prefix frontend build 通过

verification_commands:
  - pnpm --prefix frontend build

files_likely_to_change:
  - frontend/src/components/panels/ValidatePanel.jsx
  - frontend/src/tauri.js
  - docs/versions/1.1.4/更新日志.md
  - docs/versions/1.1.4/RELEASE-NOTES.md

risks:
  - 动态规则行的状态管理：用 Form.List 或手动 useState 数组；推荐 Form.List（antd 原生支持动态增删行 + getFieldsValue 直接拿结构化数据）
  - 跨字段配置只在 idcard-validate 规则行展开：需条件渲染（Form.Item 的 shouldUpdate 或 dependencies）
  - 校验规则列表来自 listRules（异步），需 useEffect 加载 + filter kind=validate
  - phone-prefixes 全局应用于 phone-validate 规则（不是每行单独配）

depends_on: [T68]
status: planned
```

## 详细规格

### 1. ValidatePanel.jsx 重做（核心骨架）

```jsx
import { useEffect, useState } from "react";
import { Button, Form, Select, Space, Typography, message } from "antd";
import { PlusOutlined, DeleteOutlined } from "@ant-design/icons";
import { useAppContext } from "../../state";
import { listRules, getSheetData, validateMultiRulesToTwoSheets } from "../../tauri";
import { PAGE_SIZE } from "../../constants";

export default function ValidatePanel() {
  const { state, dispatch, addSheetFromParse } = useAppContext();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [rules, setRules] = useState([]);

  useEffect(() => {
    listRules().then((all) => setRules(all.filter((r) => r.kind === "validate")))
      .catch((e) => console.error("listRules failed:", e));
  }, []);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];
  const ruleOptions = rules.map((r) => ({ label: r.name, value: r.id }));

  const handleValidate = async () => {
    if (!sheet) { message.warning("请先导入数据"); return; }
    const values = form.getFieldsValue(true);
    const ruleRows = values.rules || [];
    if (!ruleRows.length || ruleRows.every((r) => !r?.column || !r?.ruleId)) {
      message.warning("请至少添加一条校验规则"); return;
    }
    const multiRules = ruleRows
      .filter((r) => r?.column && r?.ruleId)
      .map((r) => ({
        column: r.column,
        ruleId: r.ruleId,
        crossField: r.ruleId === "idcard-validate" && (r.checkSex || r.checkBirth) ? {
          checkSex: !!r.checkSex,
          sexColumn: r.sexColumn || null,
          checkBirth: !!r.checkBirth,
          birthColumn: r.birthColumn || null,
        } : null,
      }));
    const phonePrefixes = (values.phonePrefixes || []).filter((p) => String(p).trim().length === 3);
    setLoading(true);
    try {
      const res = await validateMultiRulesToTwoSheets(sheet.id, sheet.sessionId, multiRules, phonePrefixes);
      // 双 Tab 落地（复用 landSheet）
      const landSheet = async (parse, name, columnHint) => { /* ... */ };
      const srcName = sheet.name || `Sheet ${sheet.id}`;
      await landSheet(res.validSheet, `${srcName}_校验通过`, "valid");
      await landSheet(res.invalidSheet, `${srcName}_校验失败`, "invalid");
      // 汇总消息
      const validCount = res.validSheet.rowCount ?? 0;
      const invalidCount = res.invalidSheet.rowCount ?? 0;
      let summary = `校验完成：${validCount} 行通过，${invalidCount} 行失败`;
      if (res.invalidReasons?.length) {
        const tally = {};
        for (const r of res.invalidReasons) tally[r.field] = (tally[r.field] || 0) + 1;
        const top = Object.entries(tally).sort((a, b) => b[1] - a[1]).slice(0, 3)
          .map(([f, n]) => `${f} ×${n}`).join("，");
        summary += `（${top}）`;
      }
      message.success(summary);
    } catch (e) {
      console.error("validate_multi_rules failed:", e);
      message.error(`校验失败：${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 4 }}>
      <Form form={form} layout="vertical" size="small">
        <Form.List name="rules" initialValue={[{}]}>
          {(fields, { add, remove }) => (
            <>
              {fields.map(({ key, name }) => (
                <div key={key} style={{ borderBottom: "1px solid #f0f0f0", paddingBottom: 8, marginBottom: 8 }}>
                  <Space align="baseline" style={{ width: "100%" }}>
                    <Form.Item name={[name, "column"]} style={{ flex: 1 }}>
                      <Select placeholder="选择列" options={headers.map((h) => ({ label: h, value: h }))} showSearch optionFilterProp="label" />
                    </Form.Item>
                    <Form.Item name={[name, "ruleId"]} style={{ flex: 1 }}>
                      <Select placeholder="选择规则" options={ruleOptions} showSearch optionFilterProp="label" />
                    </Form.Item>
                    <Button icon={<DeleteOutlined />} onClick={() => remove(name)} size="small" />
                  </Space>
                  {/* 跨字段配置：仅 idcard-validate 时展开 */}
                  <Form.Item shouldUpdate={(prev, cur) => prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId} noStyle>
                    {({ getFieldValue }) => getFieldValue(["rules", name, "ruleId"]) === "idcard-validate" ? (
                      <Space direction="vertical" style={{ width: "100%", marginTop: 4 }}>
                        <Space>
                          <Form.Item name={[name, "checkSex"]} valuePropName="checked" noStyle><checkbox>对比性别一致性</checkbox></Form.Item>
                          <Form.Item name={[name, "sexColumn"]} noStyle><Select placeholder="性别列" options={headers.map((h) => ({ label: h, value: h }))} showSearch allowClear /></Form.Item>
                        </Space>
                        <Space>
                          <Form.Item name={[name, "checkBirth"]} valuePropName="checked" noStyle><Checkbox>对比出生日期一致性</Checkbox></Form.Item>
                          <Form.Item name={[name, "birthColumn"]} noStyle><Select placeholder="出生日期列" options={headers.map((h) => ({ label: h, value: h }))} showSearch allowClear /></Form.Item>
                        </Space>
                      </Space>
                    ) : null}
                  </Form.Item>
                </div>
              ))}
              <Button type="dashed" block icon={<PlusOutlined />} onClick={() => add({})}>添加规则</Button>
            </>
          )}
        </Form.List>
        <Form.Item label="手机号前缀白名单" name="phonePrefixes" extra="可选：填三位数字前缀（如 134 / 159），应用于手机号校验规则">
          <Select mode="tags" placeholder="如 134、159（回车添加）" tokenSeparators={[",", "，"]} maxTagCount={3} />
        </Form.Item>
        <Form.Item>
          <Space direction="vertical" style={{ width: "100%" }}>
            <Button block type="primary" loading={loading} onClick={handleValidate}>校验</Button>
            <Button block onClick={() => form.resetFields()}>重置</Button>
          </Space>
        </Form.Item>
      </Form>
      <Typography.Text type="secondary" style={{ fontSize: 12 }}>
        添加多条「列 + 校验规则」组合，一个按钮校验。通过/失败的行分别写入两个新 Tab（保留原列，不新增列）。身份证规则可勾选跨字段比对性别/出生日期。
      </Typography.Text>
    </div>
  );
}
```

注意：上面骨架用 `<checkbox>` 占位，实际用 antd `<Checkbox>` 组件（需 import）。

### 2. tauri.js wrapper

```js
export function validateMultiRulesToTwoSheets(sheetId, sessionId, rules, phonePrefixes = []) {
  return invoke("validate_multi_rules_to_two_sheets", {
    sheetId, sessionId, rules, phonePrefixes,
  });
}
```

### 3. 文档重写

- `docs/versions/1.1.4/更新日志.md`：记录 T67（后端规则系统扩展）+ T68（新命令）+ T69（前端统一校验页面），删除初版的「Tabs 双页」描述
- `docs/versions/1.1.4/RELEASE-NOTES.md`：聚焦「统一校验页面：自由选多规则 + 一按钮 + 双 Tab 输出」
