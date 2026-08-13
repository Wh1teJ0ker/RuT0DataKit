import { useMemo, useState } from "react";
import { Button, Form, List, Select, Space, Tag, Typography, message } from "antd";
import { useAppContext } from "../../state";
import { extractValidateToNewSheet } from "../../tauri";
import { useRules } from "../../hooks/useRules";
import { useSheetOps } from "../../hooks/useSheetOps";
import ColumnSelect from "../shared/ColumnSelect";
import PhonePrefixSelect from "../shared/PhonePrefixSelect";

const { Text } = Typography;

// v1.1.0 提取面板：选择列 + 多选提取规则 → 按规则 pattern 对当前页数据
// 做正则提取 → 命中行高亮 hit + 命中列表。前端纯逻辑，不写 DB。
// v1.1.3 T55：新增「提取并校验到新 Tab」按钮 → 后端提取 + 函数式严格校验
// （Luhn/IPv4/IPv6/手机前缀）→ 结果落到新 Tab。
// v1.1.3 T55c：idcard-extract 规则支持性别联合校验——用户在提取时从当前 sheet
// headers 选择一个性别列，后端比对身份证第 17 位推断性别与该列值，矛盾判无效。
// v1.1.3 T56：移除「仅支持单条规则」限制——多选若干 extract 规则一次批量提取
// 到同一个新 Tab；新 Tab 输出改为纯两列 [类型, 数据值]（类型标签 = 规则名去掉
// 「提取」后缀），只写有效候选。性别列在批量含 idcard-extract 时仍显示。
// v1.2.0 T94：useRules 替代内联 listRules useEffect，landNewSheet 替代手动
// getSheetData + dispatch，ColumnSelect / PhonePrefixSelect 替代重复 Select。
export default function ExtractPanel() {
  const { state, applyRowStatuses, dispatch, addSheetFromParse } = useAppContext();
  const { rules } = useRules("extract", { patternOnly: true });
  const { landNewSheet } = useSheetOps(dispatch, addSheetFromParse);
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [extracting, setExtracting] = useState(false);
  const [hits, setHits] = useState([]);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];
  const rows = sheet?.rows || [];

  const ruleById = useMemo(
    () => Object.fromEntries(rules.map((r) => [r.id, r])),
    [rules]
  );

  // T55c：当前选中的规则里是否含 idcard-extract（用于显示性别列下拉）。
  // T56：改为 .some() 批量判定（去掉 length === 1 限制）。
  // T78：判定是否含 phone-extract（用于显示前缀白名单输入）。
  const selectedRuleIds = Form.useWatch("ruleIds", form) || [];
  const isIdcardExtract = selectedRuleIds.some(
    (id) => ruleById[id]?.params?.validator === "idcard"
  );
  const isPhoneExtract = selectedRuleIds.some((id) => id === "phone-extract");

  const handleRun = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = form.getFieldValue("column");
    const ruleIds = form.getFieldValue("ruleIds") || [];
    if (!column) {
      message.warning("请选择要提取的列");
      return;
    }
    if (!ruleIds.length) {
      message.warning("请选择至少一条提取规则");
      return;
    }
    setLoading(true);
    setHits([]);
    try {
      const page = sheet.page || 1;
      const base = (page - 1) * (sheet.pageSize || 50);
      const rowStatuses = {};
      const flatHits = [];
      rows.forEach((r, i) => {
        const value = r[column];
        if (value == null || value === "") return;
        const input = String(value);
        ruleIds.forEach((rid) => {
          const rule = ruleById[rid];
          if (!rule || !rule.pattern) return;
          let re;
          try {
            re = new RegExp(rule.pattern, "g");
          } catch (e) {
            // eslint-disable-next-line no-console
            console.error("bad regex:", rule.pattern, e);
            return;
          }
          const matches = [...input.matchAll(re)];
          matches.forEach((m) => {
            flatHits.push({
              key: `${r.key}-${rid}-${m.index}`,
              rowKey: r.key,
              rowIdx: base + i + 1,
              ruleId: rid,
              ruleName: rule.name,
              value: m[0],
              start: m.index,
              end: m.index + m[0].length,
            });
          });
        });
      });
      flatHits.forEach((h) => {
        rowStatuses[h.rowKey] = "hit";
      });
      if (Object.keys(rowStatuses).length > 0) {
        applyRowStatuses({ sheetId: sheet.id, rowStatuses });
      }
      setHits(flatHits);
      message.success(`提取完成：${flatHits.length} 个命中`);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("extract failed:", e);
      message.error(`提取失败：${e}`);
    } finally {
      setLoading(false);
    }
  };

  // v1.1.3 T55：提取 + 函数式严格校验 → 新 Tab（[类型, 数据值]，只写有效候选）。
  // 与「执行提取」（当前页预览高亮）互补：本按钮把全列候选 + 校验结果落到新 Tab，
  // 用户可在新 Tab 里筛选后导出。T56：支持批量多规则，不再限制单条。
  const handleExtractValidate = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = form.getFieldValue("column");
    const ruleIds = form.getFieldValue("ruleIds") || [];
    if (!column) {
      message.warning("请选择要提取的列");
      return;
    }
    if (!ruleIds.length) {
      message.warning("请至少选择一条提取规则");
      return;
    }
    // T55c/T56：性别列仅在批量含 idcard-extract 时使用。
    const genderCol = isIdcardExtract
      ? form.getFieldValue("genderCol") || null
      : null;
    // T78：手机号前缀白名单仅在含 phone-extract 时使用。只保留三位纯数字。
    const phonePrefixes = isPhoneExtract
      ? (form.getFieldValue("phonePrefixes") || []).filter((p) =>
          /^\d{3}$/.test(String(p).trim())
        )
      : [];
    setExtracting(true);
    try {
      const res = await extractValidateToNewSheet(
        sheet.id,
        column,
        ruleIds,
        sheet.sessionId,
        genderCol,
        phonePrefixes
      );
      await landNewSheet(res, `${column}_提取`, column, sheet.sessionId);
      message.success(
        `提取完成：${res.rowCount} 条，跳过 ${res.skipped ?? 0}`
      );
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("extract_validate_to_new_sheet failed:", e);
      message.error(`提取并校验失败：${e}`);
    } finally {
      setExtracting(false);
    }
  };

  return (
    <div style={{ padding: 4 }}>
      <Form form={form} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
          <ColumnSelect headers={headers} placeholder="选择列" />
        </Form.Item>
        <Form.Item label="提取规则（可多选）" name="ruleIds">
          <Select
            mode="multiple"
            placeholder="选择规则"
            options={rules.map((r) => ({ label: r.name, value: r.id }))}
            notFoundContent="无可用规则"
          />
        </Form.Item>
        {isIdcardExtract && (
          <Form.Item
            label="性别列（联合校验）"
            name="genderCol"
            extra="比对身份证推断性别与该列，矛盾判无效"
          >
            <ColumnSelect
              headers={headers}
              placeholder="选择性别列"
              allowClear
            />
          </Form.Item>
        )}
        {isPhoneExtract && (
          <Form.Item
            label="手机号前缀白名单"
            name="phonePrefixes"
            extra="三位数字前缀，留空=不限"
          >
            <PhonePrefixSelect />
          </Form.Item>
        )}
        <Form.Item>
          <Space direction="vertical" style={{ width: "100%" }}>
            <Space>
              <Button type="primary" loading={loading} onClick={handleRun}>
                执行提取
              </Button>
              <Button
                onClick={() => {
                  form.resetFields();
                  setHits([]);
                }}
              >
                重置
              </Button>
            </Space>
            <Button
              block
              loading={extracting}
              onClick={handleExtractValidate}
            >
              提取并校验到新 Tab
            </Button>
          </Space>
        </Form.Item>
      </Form>
      {hits.length > 0 && (
        <div style={{ marginTop: 8 }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            提取结果（{hits.length} 项）：
          </Text>
          <List
            size="small"
            bordered
            style={{ marginTop: 4, maxHeight: "40vh", overflow: "auto" }}
            dataSource={hits}
            renderItem={(h) => (
              <List.Item style={{ padding: "4px 8px" }}>
                <Tag color="blue" style={{ marginRight: 8 }}>
                  {h.ruleName}
                </Tag>
                <Text style={{ fontSize: 12 }}>
                  行 {h.rowIdx}：{h.value}
                </Text>
              </List.Item>
            )}
          />
        </div>
      )}
    </div>
  );
}
