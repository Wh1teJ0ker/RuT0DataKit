import { useEffect, useState } from "react";
import { Button, Form, Select, Space, Tabs, Typography, message } from "antd";
import { useAppContext } from "../../state";
import {
  validateColumn,
  listRules,
  getSheetData,
  validateRowsToTwoSheets,
} from "../../tauri";
import { PAGE_SIZE } from "../../constants";

const { Text } = Typography;

// v1.1.0 单列校验面板：选择列 + 规则 → 校验 → 不通过行原位高亮 invalid。
// v1.1.3 T57：新增行级多字段校验（7 字段 + 跨字段联合 → 双 Tab）。
// v1.1.4 T67：行级校验从独立能力（RowValidatePanel）合并入「校验」模块，
//   与单列校验并列为同一面板的两个 Tab，移除 TopToolbar/SidePanel 的 rowValidate 入口。

// 行级校验字段定义：label / form key / 用户友好的规则说明。
const ROW_FIELDS = [
  { key: "username", label: "用户名", hint: "纯字母数字（admin / lufe1jian / 91xxev）" },
  { key: "name", label: "姓名", hint: "2-4 位中文（张三 / 李四）" },
  { key: "sex", label: "性别", hint: "男 / 女；与身份证第 17 位奇偶比对" },
  { key: "birth", label: "出生日期", hint: "8 位数字；与身份证第 7-14 位比对" },
  { key: "idcard", label: "身份证号", hint: "18 位 GB 11643-1999 校验码" },
  { key: "phone", label: "手机号", hint: "11 位、1 开头；可配前缀白名单" },
  { key: "address", label: "地址", hint: "全中文 + 号(1-1500) + 室(101-999)" },
];

export default function ValidatePanel() {
  const { state, applyRowStatuses, dispatch, addSheetFromParse } = useAppContext();

  // 单列校验表单
  const [colForm] = Form.useForm();
  const [colLoading, setColLoading] = useState(false);
  const [rules, setRules] = useState([]);

  // 行级校验表单
  const [rowForm] = Form.useForm();
  const [rowLoading, setRowLoading] = useState(false);

  useEffect(() => {
    let alive = true;
    listRules()
      .then((all) => {
        if (!alive) return;
        setRules(all.filter((r) => r.kind === "validate"));
      })
      .catch((e) => {
        // eslint-disable-next-line no-console
        console.error("listRules failed:", e);
      });
    return () => {
      alive = false;
    };
  }, []);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];

  // ===== 单列校验：validateColumn → applyRowStatuses 原位高亮 =====
  const handleRunColumn = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = colForm.getFieldValue("column");
    const ruleId = colForm.getFieldValue("ruleId");
    if (!column) {
      message.warning("请选择要校验的列");
      return;
    }
    if (!ruleId) {
      message.warning("请选择校验规则");
      return;
    }
    setColLoading(true);
    try {
      // validate_column 返回 Vec<RowValidation>（直接是数组，不是 { results: [...] }）。
      // rowIdx 是 DB 绝对行号（row_idx=0 是表头行，数据行从 1 开始），与
      // reducer.APPLY_SEARCH_HITS 的换算保持一致：
      //   pageInnerIdx = rowIdx - 1 - pageBase
      //   rowKey = `${sheetId}-${page}-${pageInnerIdx}`
      const results = await validateColumn(sheet.id, column, ruleId);
      const page = sheet.page || 1;
      const base = (page - 1) * (sheet.pageSize || 50);
      const rowStatuses = {};
      let failedCount = 0;
      (Array.isArray(results) ? results : []).forEach((r) => {
        if (!r.passed) {
          failedCount += 1;
          const i = r.rowIdx - 1 - base;
          if (i >= 0) {
            rowStatuses[`${sheet.id}-${page}-${i}`] = "invalid";
          }
        }
      });
      if (Object.keys(rowStatuses).length > 0) {
        applyRowStatuses({ sheetId: sheet.id, rowStatuses });
      }
      message.success(`校验完成：${failedCount} 行不通过`);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("validate_column failed:", e);
      message.error(`校验失败：${e}`);
    } finally {
      setColLoading(false);
    }
  };

  // ===== 行级校验：validateRowsToTwoSheets → 双 Tab 落地 =====
  const handleRunRow = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const values = rowForm.getFieldsValue(true);
    // 收集映射：至少 1 个字段映射到非空列名。
    const fieldColumns = {};
    let mapped = 0;
    for (const f of ROW_FIELDS) {
      const v = values[f.key];
      if (v && String(v).trim()) {
        fieldColumns[f.key] = String(v).trim();
        mapped += 1;
      }
    }
    if (mapped === 0) {
      message.warning("请至少为一个字段选择对应的列");
      return;
    }
    const phonePrefixes = (values.phonePrefixes || []).filter(
      (p) => String(p).trim().length === 3
    );
    setRowLoading(true);
    try {
      const res = await validateRowsToTwoSheets(
        sheet.id,
        sheet.sessionId,
        fieldColumns,
        phonePrefixes
      );
      // 双 Tab 落地：valid + invalid 各走 addSheetFromParse + getSheetData + SET_SHEET_DATA。
      // 名称沿用后端的 `{name}_校验通过` / `{name}_校验失败`，这里用 column 字段
      // 透传 newSheetId / headers / rowCount 即可（addSheetFromParse 用 newSheetId
      // 作为 sheet.id）。
      const landSheet = async (parse, name, columnHint) => {
        addSheetFromParse({
          newSheetId: parse.newSheetId,
          headers: parse.headers,
          rowCount: parse.rowCount,
          skipped: parse.skipped ?? 0,
          sessionId: sheet.sessionId,
          column: columnHint,
          name,
        });
        const data = await getSheetData(parse.newSheetId, 1, PAGE_SIZE);
        dispatch({
          type: "SET_SHEET_DATA",
          payload: { ...data, sheetId: parse.newSheetId },
        });
      };
      const srcName = sheet.name || `Sheet ${sheet.id}`;
      await landSheet(res.validSheet, `${srcName}_校验通过`, "valid");
      await landSheet(res.invalidSheet, `${srcName}_校验失败`, "invalid");
      // 汇总消息：通过 / 失败行数 + top 失败原因。
      const validCount = res.validSheet.rowCount ?? 0;
      const invalidCount = res.invalidSheet.rowCount ?? 0;
      let summary = `校验完成：${validCount} 行通过，${invalidCount} 行失败`;
      if (res.invalidReasons && res.invalidReasons.length) {
        const tally = {};
        for (const r of res.invalidReasons) {
          tally[r.field] = (tally[r.field] || 0) + 1;
        }
        const top = Object.entries(tally)
          .sort((a, b) => b[1] - a[1])
          .slice(0, 3)
          .map(([f, n]) => `${f} ×${n}`)
          .join("，");
        summary += `（${top}）`;
      }
      message.success(summary);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("validate_rows_to_two_sheets failed:", e);
      message.error(`行级校验失败：${e}`);
    } finally {
      setRowLoading(false);
    }
  };

  // 单列校验 Tab 内容
  const columnTab = (
    <Form form={colForm} layout="vertical" size="small">
      <Form.Item label="目标列" name="column">
        <Select
          placeholder="选择要校验的列"
          options={headers.map((h) => ({ label: h, value: h }))}
          showSearch
          optionFilterProp="label"
        />
      </Form.Item>
      <Form.Item label="校验规则" name="ruleId">
        <Select
          placeholder="选择校验规则"
          options={rules.map((r) => ({
            label: r.name,
            value: r.id,
          }))}
          notFoundContent="无可用规则"
        />
      </Form.Item>
      <Form.Item>
        <Space>
          <Button type="primary" loading={colLoading} onClick={handleRunColumn}>
            执行校验
          </Button>
          <Button
            onClick={() => {
              colForm.resetFields();
            }}
          >
            重置
          </Button>
        </Space>
      </Form.Item>
    </Form>
  );

  // 行级校验 Tab 内容
  const rowTab = (
    <Form form={rowForm} layout="vertical" size="small">
      {ROW_FIELDS.map((f) => (
        <Form.Item key={f.key} label={f.label} name={f.key} extra={f.hint}>
          <Select
            placeholder={`选择 ${f.label} 对应的列（可留空）`}
            options={headers.map((h) => ({ label: h, value: h }))}
            allowClear
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
      ))}
      <Form.Item
        label="手机号前缀白名单"
        name="phonePrefixes"
        extra="可选：填三位数字前缀（如 134 / 159），空 = 仅检查 1 开头 + 11 位"
      >
        <Select
          mode="tags"
          placeholder="如 134、159（回车添加）"
          tokenSeparators={[",", "，"]}
          maxTagCount={3}
        />
      </Form.Item>
      <Form.Item>
        <Space direction="vertical" style={{ width: "100%" }}>
          <Button block type="primary" loading={rowLoading} onClick={handleRunRow}>
            行级校验并分流到双 Tab
          </Button>
          <Button
            block
            onClick={() => {
              rowForm.resetFields();
            }}
          >
            重置
          </Button>
        </Space>
      </Form.Item>
      <Text type="secondary" style={{ fontSize: 12 }}>
        校验通过 / 失败的行分别写入两个新 Tab（保留原列，不新增列）。跨字段
        联合校验（性别 / 出生日期 vs 身份证号）仅当相关字段都映射时生效。
      </Text>
    </Form>
  );

  return (
    <div style={{ padding: 4 }}>
      <Tabs
        defaultActiveKey="column"
        size="small"
        items={[
          { key: "column", label: "单列校验", children: columnTab },
          { key: "row", label: "行级校验", children: rowTab },
        ]}
      />
    </div>
  );
}
