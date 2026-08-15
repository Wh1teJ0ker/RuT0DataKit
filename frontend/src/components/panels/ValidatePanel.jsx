import { useState } from "react";
import { Button, Form, Select, Space, Typography, message } from "antd";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { useAppContext } from "../../state";
import { validateMultiRulesToTwoSheets } from "../../tauri";
// v1.2.0 T94：useRules 替代内联 listRules useEffect，useSheetOps.landNewSheet
// 替代手动 getSheetData + dispatch 双 Tab 落地。ColumnSelect 替代重复列 Select。
import { useRules } from "../../hooks/useRules";
import { useSheetOps } from "../../hooks/useSheetOps";
import { useActiveSheet } from "../../hooks/useActiveSheet";
import { normalizePhonePrefixes } from "../../utils/phonePrefix";
import ColumnSelect from "../shared/ColumnSelect";
// v1.2.1 T13：契约组装移到 validate/contractBuilder.js，行级参数组件移到
// validate/RuleRowParams.jsx。
import { buildMultiRules } from "./validate/contractBuilder";
import { RuleRowParams } from "./validate/RuleRowParams";

const { Text } = Typography;

// v1.1.0 单列校验面板：选择列 + 规则 → 校验 → 不通过行原位高亮 invalid。
// v1.1.3 T57：新增行级多字段校验（7 字段 + 跨字段联合 → 双 Tab）。
// v1.1.4 T67：行级校验从独立能力合并入「校验」模块（Tabs 双页）。
// v1.1.4 T69：重做为统一表单（非 Tabs）— 用户自由组合多条「列 + 校验规则」，
//   一个「校验」按钮 → 调 validate_multi_rules_to_two_sheets → 双 Tab 落地。
//   身份证规则行可勾选跨字段比对性别 / 出生日期列。
// v1.2.0 T94：useRules 替代 listRules useEffect，useSheetOps.landNewSheet
//   替代手动双 Tab getSheetData + dispatch。
// v1.2.1 T13：契约组装 + 行级参数渲染拆为子模块，主文件瘦身。
export default function ValidatePanel() {
  const { dispatch } = useAppContext();
  const { rules } = useRules("validate");
  const { landNewSheet } = useSheetOps(dispatch);
  const { sheet, headers } = useActiveSheet();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const ruleOptions = rules.map((r) => ({ label: r.name, value: r.id }));

  const handleValidate = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const values = form.getFieldsValue(true);
    const ruleRows = values.rules || [];
    if (!ruleRows.length || ruleRows.every((r) => !r?.column || !r?.ruleId)) {
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
    const multiRules = buildMultiRules(ruleRows);
    // T78：手机号前缀白名单改为每行内嵌（选中 phone-validate 时展开）。
    // 多行都选 phone-validate 时合并所有行前缀（去重），只保留三位纯数字。
    const phonePrefixes = normalizePhonePrefixes(
      Array.from(
        new Set(
          ruleRows
            .filter((r) => r?.ruleId === "phone-validate")
            .flatMap((r) => r?.phonePrefixes || [])
        )
      )
    );
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
    <div style={{ height: "100%", overflow: "auto", padding: 4 }}>
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
                    <Form.Item name={[name, "column"]} style={{ flex: 1, marginBottom: 0 }}>
                      <ColumnSelect headers={headers} placeholder="选择列" />
                    </Form.Item>
                    <Form.Item name={[name, "ruleId"]} style={{ flex: 1, marginBottom: 0 }}>
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
                  <RuleRowParams name={name} headers={headers} />
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
