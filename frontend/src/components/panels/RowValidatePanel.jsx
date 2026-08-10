import { useState } from "react";
import { Button, Form, Select, Space, Typography, message } from "antd";
import { useAppContext } from "../../state";
import { getSheetData, validateRowsToTwoSheets } from "../../tauri";
import { PAGE_SIZE } from "../../constants";

const { Text } = Typography;

// v1.1.3 T57：行级多字段校验面板。
//
// 7 个字段（username/name/sex/birth/idcard/phone/address）各自映射到当前 sheet
// 的某个列表头；未映射的字段（留空）不校验。手机号可填三位前缀白名单（空 = 仅
// 检查 1 开头 + 11 位）。跨字段联合校验（sex vs idcard 第 17 位性别推断、
// birth vs idcard 第 7-14 位出生日期码）仅当相关字段都映射且 idcard 本身有效
// 时生效，无需用户额外指定。
//
// 校验后由后端把通过 / 失败的行分别写入两个新 Tab（`{name}_校验通过` /
// `{name}_校验失败`），保留原列不新增列。前端对两个新 Tab 分别走
// `addSheetFromParse` + `getSheetData` + `SET_SHEET_DATA` 落地，与 ExtractPanel
// 的 `extract_validate_to_new_sheet` 落地流程对称。

// 字段定义：label / form key / 用户友好的规则说明。
const FIELDS = [
  { key: "username", label: "用户名", hint: "纯字母数字（admin / lufe1jian / 91xxev）" },
  { key: "name", label: "姓名", hint: "2-4 位中文（张三 / 李四）" },
  { key: "sex", label: "性别", hint: "男 / 女；与身份证第 17 位奇偶比对" },
  { key: "birth", label: "出生日期", hint: "8 位数字；与身份证第 7-14 位比对" },
  { key: "idcard", label: "身份证号", hint: "18 位 GB 11643-1999 校验码" },
  { key: "phone", label: "手机号", hint: "11 位、1 开头；可配前缀白名单" },
  { key: "address", label: "地址", hint: "全中文 + 号(1-1500) + 室(101-999)" },
];

export default function RowValidatePanel() {
  const { state, dispatch, addSheetFromParse } = useAppContext();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];

  const handleValidate = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const values = form.getFieldsValue(true);
    // 收集映射：至少 1 个字段映射到非空列名。
    const fieldColumns = {};
    let mapped = 0;
    for (const f of FIELDS) {
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
    setLoading(true);
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
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 4 }}>
      <Form form={form} layout="vertical" size="small">
        {FIELDS.map((f) => (
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
            <Button block type="primary" loading={loading} onClick={handleValidate}>
              行级校验并分流到双 Tab
            </Button>
            <Button
              block
              onClick={() => {
                form.resetFields();
              }}
            >
              重置
            </Button>
          </Space>
        </Form.Item>
      </Form>
      <Text type="secondary" style={{ fontSize: 12 }}>
        校验通过 / 失败的行分别写入两个新 Tab（保留原列，不新增列）。跨字段
        联合校验（性别 / 出生日期 vs 身份证号）仅当相关字段都映射时生效。
      </Text>
    </div>
  );
}
