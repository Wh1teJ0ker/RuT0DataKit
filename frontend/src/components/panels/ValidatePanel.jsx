import { useState } from "react";
import {
  Button,
  Checkbox,
  Form,
  Input,
  InputNumber,
  Select,
  Space,
  Typography,
  message,
} from "antd";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { useAppContext } from "../../state";
import {
  validateMultiRulesToTwoSheets,
} from "../../tauri";
// v1.2.0 T94：useRules 替代内联 listRules useEffect，useSheetOps.landNewSheet
// 替代手动 getSheetData + dispatch 双 Tab 落地。ColumnSelect 替代重复列 Select。
import { useRules } from "../../hooks/useRules";
import { useSheetOps } from "../../hooks/useSheetOps";
import ColumnSelect from "../shared/ColumnSelect";
import PhonePrefixSelect from "../shared/PhonePrefixSelect";
// v1.1.4 续轮 T71：generic-validate 行级参数共享模块。
// 行内字段直接由 Form.List 收集，组装 multiRules 时把 charClasses 数组映射为
// allowDigits/allowLetters 布尔 + specialChars 白名单字符串，minLen/maxLen 直接回传。
// buildGenericParamsForRun 用于构造与后端 ExtractParams::Generic 对齐的对象。
// T77：allowSpecial bool → allowSpecialChars string（自定义特殊字符白名单）。
import { buildGenericParamsForRun } from "./validateParams";

const { Text } = Typography;

// v1.1.5 T88：birth-validate 行级生日格式勾选。
// 全不选 = 接受所有格式（默认，向后兼容）；勾选后仅校验勾选的格式。
const BIRTH_FORMATS = [
  { label: "纯数字 yyyymmdd", value: "yyyymmdd" },
  { label: "连字符 yyyy-mm-dd", value: "yyyy-mm-dd" },
  { label: "斜杠 yyyy/mm/dd", value: "yyyy/mm/dd" },
  { label: "点号 yyyy.mm.dd", value: "yyyy.mm.dd" },
];

// v1.1.0 单列校验面板：选择列 + 规则 → 校验 → 不通过行原位高亮 invalid。
// v1.1.3 T57：新增行级多字段校验（7 字段 + 跨字段联合 → 双 Tab）。
// v1.1.4 T67：行级校验从独立能力合并入「校验」模块（Tabs 双页）。
// v1.1.4 T69：重做为统一表单（非 Tabs）— 用户自由组合多条「列 + 校验规则」，
//   一个「校验」按钮 → 调 validate_multi_rules_to_two_sheets → 双 Tab 落地。
//   身份证规则行可勾选跨字段比对性别 / 出生日期列。
// v1.2.0 T94：useRules 替代 listRules useEffect，useSheetOps.landNewSheet
//   替代手动双 Tab getSheetData + dispatch。

export default function ValidatePanel() {
  const { state, dispatch, addSheetFromParse } = useAppContext();
  const { rules } = useRules("validate");
  const { landNewSheet } = useSheetOps(dispatch, addSheetFromParse);
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];
  const ruleOptions = rules.map((r) => ({ label: r.name, value: r.id }));

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
    // 组装后端契约：每条 { column, ruleId, crossField?, paramsOverride? }。
    // crossField 仅在 idcard-validate 且勾选了性别/出生比对时传，否则传 null。
    // paramsOverride 仅在 generic-validate 时携带，覆盖 DB 默认 params
    // （字符类 + 长度限制），其他规则传 null（后端忽略）。
    const multiRules = ruleRows
      .filter((r) => r?.column && r?.ruleId)
      .map((r) => {
        const item = {
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
          paramsOverride: null,
        };
        // v1.1.4 续轮 T71：generic-validate 行附带 paramsOverride。
        // T77：特殊符号从布尔全开/全关改为自定义白名单字符串。
        if (r.ruleId === "generic-validate") {
          const classes = r.charClasses || [];
          item.paramsOverride = buildGenericParamsForRun({
            allowDigits: classes.includes("digits"),
            allowLetters: classes.includes("letters"),
            allowSpecialChars: r.specialChars || "",
            minLen: r.minLen ?? null,
            maxLen: r.maxLen ?? null,
          });
        }
        // v1.1.5 T88：birth-validate 行附带 paramsOverride，仅勾选了格式时携带。
        // 全不选 = 接受所有格式（默认，向后兼容），不传 paramsOverride，沿用 DB 默认。
        if (r.ruleId === "birth-validate" && r.birthFormats?.length) {
          item.paramsOverride = { validator: "birth", formats: r.birthFormats };
        }
        return item;
      });
    // T78：手机号前缀白名单改为每行内嵌（选中 phone-validate 时展开）。
    // 多行都选 phone-validate 时合并所有行前缀（去重），只保留三位纯数字。
    const phonePrefixes = Array.from(
      new Set(
        ruleRows
          .filter((r) => r?.ruleId === "phone-validate")
          .flatMap((r) => r?.phonePrefixes || [])
      )
    ).filter((p) => /^\d{3}$/.test(String(p).trim()));
    setLoading(true);
    try {
      const res = await validateMultiRulesToTwoSheets(
        sheet.id,
        sheet.sessionId,
        multiRules,
        phonePrefixes,
      );
      // 双 Tab 落地：valid + invalid 各走 landNewSheet。
      const srcName = sheet.name || `Sheet ${sheet.id}`;
      await landNewSheet(res.validSheet, `${srcName}_校验通过`, "valid", sheet.sessionId);
      await landNewSheet(res.invalidSheet, `${srcName}_校验失败`, "invalid", sheet.sessionId);
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
                      <ColumnSelect
                        headers={headers}
                        placeholder="选择列"
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
                              <ColumnSelect
                                headers={headers}
                                placeholder="性别列"
                                allowClear
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
                              <ColumnSelect
                                headers={headers}
                                placeholder="出生日期列"
                                allowClear
                              />
                            </Form.Item>
                          </Space>
                        </Space>
                      ) : null
                    }
                  </Form.Item>
                  {/* v1.1.4 续轮 T71：generic-validate 行级参数配置。
                      用 shouldUpdate 监听该行 ruleId 变化，仅 generic-validate 时展开。
                      字段名 [name, "charClasses"] / [name, "specialChars"] / [name, "minLen"] / [name, "maxLen"]
                      与下方 multiRules 组装逻辑对齐。 */}
                  <Form.Item
                    shouldUpdate={(prev, cur) =>
                      prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
                    }
                    noStyle
                  >
                    {({ getFieldValue }) =>
                      getFieldValue(["rules", name, "ruleId"]) ===
                      "generic-validate" ? (
                        <Space
                          direction="vertical"
                          style={{ width: "100%", marginTop: 4 }}
                        >
                          <Form.Item
                            name={[name, "charClasses"]}
                            label="允许的字符类"
                            style={{ marginBottom: 0 }}
                          >
                            <Checkbox.Group
                              options={[
                                { label: "纯数字", value: "digits" },
                                { label: "纯字母", value: "letters" },
                              ]}
                            />
                          </Form.Item>
                          <Form.Item
                            name={[name, "specialChars"]}
                            label="允许的特殊符号"
                            style={{ marginBottom: 0 }}
                          >
                            <Input
                              placeholder="留空=不允许；如 _-.@"
                              allowClear
                            />
                          </Form.Item>
                          <Space>
                            <Form.Item
                              name={[name, "minLen"]}
                              label="最小长度"
                              noStyle
                            >
                              <InputNumber
                                placeholder="不限"
                                min={0}
                                style={{ width: 100 }}
                              />
                            </Form.Item>
                            <Form.Item
                              name={[name, "maxLen"]}
                              label="最大长度"
                              noStyle
                            >
                              <InputNumber
                                placeholder="不限"
                                min={0}
                                style={{ width: 100 }}
                              />
                            </Form.Item>
                          </Space>
                        </Space>
                      ) : null
                    }
                  </Form.Item>
                  {/* T78：phone-validate 行级前缀白名单配置。
                      用 shouldUpdate 监听该行 ruleId 变化，仅 phone-validate 时展开。
                      字段名 [name, "phonePrefixes"] 与上方 handleValidate 收集逻辑对齐。 */}
                  <Form.Item
                    shouldUpdate={(prev, cur) =>
                      prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
                    }
                    noStyle
                  >
                    {({ getFieldValue }) =>
                      getFieldValue(["rules", name, "ruleId"]) ===
                      "phone-validate" ? (
                          <Form.Item
                            name={[name, "phonePrefixes"]}
                            label="手机号前缀白名单"
                            style={{ marginTop: 4, marginBottom: 0 }}
                            extra="三位数字前缀，留空=不限"
                          >
                            <PhonePrefixSelect />
                          </Form.Item>
                      ) : null
                    }
                  </Form.Item>
                  {/* v1.1.5 T88：birth-validate 行级生日格式勾选。
                      全不选 = 接受所有格式（默认，向后兼容）；勾选后仅校验勾选的格式。
                      字段名 [name, "birthFormats"] 与上方 handleValidate 组装逻辑对齐。 */}
                  <Form.Item
                    shouldUpdate={(prev, cur) =>
                      prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
                    }
                    noStyle
                  >
                    {({ getFieldValue }) =>
                      getFieldValue(["rules", name, "ruleId"]) ===
                      "birth-validate" ? (
                        <Form.Item
                          name={[name, "birthFormats"]}
                          label="生日格式"
                          style={{ marginTop: 4, marginBottom: 0 }}
                          extra="全不选=接受所有格式"
                        >
                          <Checkbox.Group options={BIRTH_FORMATS} />
                        </Form.Item>
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
        多条「列+规则」组合一次校验，通过/失败行分写两个新 Tab。
      </Text>
    </div>
  );
}
