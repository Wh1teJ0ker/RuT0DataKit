import { useEffect, useState } from "react";
import {
  Button,
  Checkbox,
  Form,
  Select,
  Space,
  Typography,
  message,
} from "antd";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { useAppContext } from "../../state";
import {
  getSheetData,
  listRules,
  validateMultiRulesToTwoSheets,
} from "../../tauri";
import { PAGE_SIZE } from "../../constants";

const { Text } = Typography;

// v1.1.0 单列校验面板：选择列 + 规则 → 校验 → 不通过行原位高亮 invalid。
// v1.1.3 T57：新增行级多字段校验（7 字段 + 跨字段联合 → 双 Tab）。
// v1.1.4 T67：行级校验从独立能力合并入「校验」模块（Tabs 双页）。
// v1.1.4 T69：重做为统一表单（非 Tabs）— 用户自由组合多条「列 + 校验规则」，
//   一个「校验」按钮 → 调 validate_multi_rules_to_two_sheets → 双 Tab 落地。
//   身份证规则行可勾选跨字段比对性别 / 出生日期列。

export default function ValidatePanel() {
  const { state, dispatch, addSheetFromParse } = useAppContext();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [rules, setRules] = useState([]);

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
  const ruleOptions = rules.map((r) => ({ label: r.name, value: r.id }));
  const columnOptions = headers.map((h) => ({ label: h, value: h }));

  const handleValidate = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const values = form.getFieldsValue(true);
    const ruleRows = values.rules || [];
    if (
      !ruleRows.length ||
      ruleRows.every((r) => !r?.column || !r?.ruleId)
    ) {
      message.warning("请至少添加一条校验规则");
      return;
    }
    // crossField 列必填校验：idcard-validate 行勾选了「对比性别/出生日期
    // 一致性」却没选对应列时，后端会静默跳过比对（sexColumn/birthColumn 为
    // null），用户以为开了比对实际被跳过。这里在组装契约前拦截。
    for (const r of ruleRows) {
      if (r?.ruleId !== "idcard-validate") continue;
      if (r.checkSex && !r.sexColumn) {
        message.warning("请选择性别列");
        return;
      }
      if (r.checkBirth && !r.birthColumn) {
        message.warning("请选择出生日期列");
        return;
      }
    }
    // 组装后端契约：每条 { column, ruleId, crossField? }，crossField 仅在
    // idcard-validate 且勾选了性别/出生比对时传，否则传 null（后端忽略）。
    const multiRules = ruleRows
      .filter((r) => r?.column && r?.ruleId)
      .map((r) => ({
        column: r.column,
        ruleId: r.ruleId,
        crossField:
          r.ruleId === "idcard-validate" && (r.checkSex || r.checkBirth)
            ? {
                checkSex: !!r.checkSex,
                sexColumn: r.sexColumn || null,
                checkBirth: !!r.checkBirth,
                birthColumn: r.birthColumn || null,
              }
            : null,
      }));
    // 手机号前缀白名单：只保留三位纯数字（"abc"/"1a3" 会被剔掉），应用于
    // phone-validate 规则（全局，不是每行单独配）。
    const phonePrefixes = (values.phonePrefixes || []).filter((p) =>
      /^\d{3}$/.test(String(p).trim()),
    );
    setLoading(true);
    try {
      const res = await validateMultiRulesToTwoSheets(
        sheet.id,
        sheet.sessionId,
        multiRules,
        phonePrefixes,
      );
      // 双 Tab 落地：valid + invalid 各走 addSheetFromParse + getSheetData +
      // SET_SHEET_DATA（复用 v1.1.3 RowValidatePanel 的 landSheet 闭包模式）。
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
      // 汇总消息：通过 / 失败行数 + top 失败原因（按 field 计数）。
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
                <div
                  key={key}
                  style={{
                    borderBottom: "1px solid #f0f0f0",
                    paddingBottom: 8,
                    marginBottom: 8,
                  }}
                >
                  <Space align="baseline" style={{ width: "100%" }}>
                    <Form.Item
                      name={[name, "column"]}
                      style={{ flex: 1, marginBottom: 0 }}
                    >
                      <Select
                        placeholder="选择列"
                        options={columnOptions}
                        showSearch
                        optionFilterProp="label"
                      />
                    </Form.Item>
                    <Form.Item
                      name={[name, "ruleId"]}
                      style={{ flex: 1, marginBottom: 0 }}
                    >
                      <Select
                        placeholder="选择规则"
                        options={ruleOptions}
                        showSearch
                        optionFilterProp="label"
                      />
                    </Form.Item>
                    <Button
                      icon={<DeleteOutlined />}
                      onClick={() => remove(name)}
                      size="small"
                    />
                  </Space>
                  {/* 跨字段配置：仅 idcard-validate 时展开。用 shouldUpdate
                      监听该行 ruleId 变化，避免渲染其他规则行。 */}
                  <Form.Item
                    shouldUpdate={(prev, cur) =>
                      prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
                    }
                    noStyle
                  >
                    {({ getFieldValue }) =>
                      getFieldValue(["rules", name, "ruleId"]) ===
                      "idcard-validate" ? (
                        <Space
                          direction="vertical"
                          style={{ width: "100%", marginTop: 4 }}
                        >
                          <Space>
                            <Form.Item
                              name={[name, "checkSex"]}
                              valuePropName="checked"
                              noStyle
                            >
                              <Checkbox>对比性别一致性</Checkbox>
                            </Form.Item>
                            <Form.Item name={[name, "sexColumn"]} noStyle>
                              <Select
                                placeholder="性别列"
                                options={columnOptions}
                                showSearch
                                allowClear
                                optionFilterProp="label"
                              />
                            </Form.Item>
                          </Space>
                          <Space>
                            <Form.Item
                              name={[name, "checkBirth"]}
                              valuePropName="checked"
                              noStyle
                            >
                              <Checkbox>对比出生日期一致性</Checkbox>
                            </Form.Item>
                            <Form.Item name={[name, "birthColumn"]} noStyle>
                              <Select
                                placeholder="出生日期列"
                                options={columnOptions}
                                showSearch
                                allowClear
                                optionFilterProp="label"
                              />
                            </Form.Item>
                          </Space>
                        </Space>
                      ) : null
                    }
                  </Form.Item>
                </div>
              ))}
              <Button
                type="dashed"
                block
                icon={<PlusOutlined />}
                onClick={() => add({})}
              >
                添加规则
              </Button>
            </>
          )}
        </Form.List>
        <Form.Item
          label="手机号前缀白名单"
          name="phonePrefixes"
          extra="可选：填三位数字前缀（如 134 / 159），应用于手机号校验规则"
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
              校验
            </Button>
            <Button block onClick={() => form.resetFields()}>
              重置
            </Button>
          </Space>
        </Form.Item>
      </Form>
      <Text type="secondary" style={{ fontSize: 12 }}>
        添加多条「列 + 校验规则」组合，一个按钮校验。通过/失败的行分别写入两个新
        Tab（保留原列，不新增列）。身份证规则可勾选跨字段比对性别/出生日期。
      </Text>
    </div>
  );
}
