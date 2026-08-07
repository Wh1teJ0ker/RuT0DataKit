import { useEffect, useState } from "react";
import { Button, Form, Input, Select, Space, message } from "antd";
import { useAppContext } from "../../state";
import { maskColumn, getSheetData, listRules, updateRuleParams } from "../../tauri";
import { PAGE_SIZE } from "../../constants";

// v1.1.0 脱敏面板：选择列 + 掩码字符（默认 *）→ 就地脱敏 → 回写 DB + 行高亮 masked。
// ≥3 字符：保留首尾，中间掩码字符替换；2 字符：保留首字符末位掩码字符；1 字符：掩码字符。
// 重置 → 默认 *；保存设置 → 持久化到 name-mask 规则。
const DEFAULT_MASK_CHAR = "*";

export default function MaskPanel() {
  const { state, dispatch, applyRowStatuses } = useAppContext();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [maskChar, setMaskChar] = useState(DEFAULT_MASK_CHAR);
  const [maskRuleId, setMaskRuleId] = useState(null);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];

  // 加载 name-mask 规则的掩码字符（None/空 → 默认 *）。
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const all = await listRules();
        const maskRule = all.find((r) => r.kind === "mask");
        if (!maskRule) return;
        if (cancelled) return;
        setMaskRuleId(maskRule.id);
        const ch =
          maskRule.replacement && maskRule.replacement.length > 0
            ? maskRule.replacement[0]
            : DEFAULT_MASK_CHAR;
        setMaskChar(ch);
        form.setFieldValue("maskChar", ch);
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("load mask rule failed:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleRun = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = form.getFieldValue("column");
    if (!column) {
      message.warning("请选择要脱敏的列");
      return;
    }
    setLoading(true);
    try {
      const ch = maskChar || DEFAULT_MASK_CHAR;
      const res = await maskColumn(sheet.id, column, maskRuleId, ch);
      const page = sheet.page || 1;
      const data = await getSheetData(sheet.id, page, sheet.pageSize || PAGE_SIZE);
      dispatch({
        type: "SET_SHEET_DATA",
        payload: { ...data, sheetId: sheet.id },
      });
      const rowStatuses = {};
      (data.rows || []).forEach((_, i) => {
        rowStatuses[`${sheet.id}-${page}-${i}`] = "masked";
      });
      applyRowStatuses({ sheetId: sheet.id, rowStatuses });
      message.success(`脱敏完成：${res.affected ?? 0} 行`);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("mask_column failed:", e);
      message.error(`脱敏失败：${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSaveSetting = async () => {
    if (!maskRuleId) {
      message.warning("未找到脱敏规则，无法保存");
      return;
    }
    setSaving(true);
    try {
      const ch = maskChar || DEFAULT_MASK_CHAR;
      await updateRuleParams(maskRuleId, null, ch);
      message.success("掩码字符已保存");
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("save mask char failed:", e);
      message.error(`保存失败：${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    setMaskChar(DEFAULT_MASK_CHAR);
    form.setFieldValue("maskChar", DEFAULT_MASK_CHAR);
  };

  return (
    <div style={{ padding: 4 }}>
      <Form form={form} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
          <Select
            placeholder="选择要脱敏的列"
            options={headers.map((h) => ({ label: h, value: h }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        <Form.Item label="掩码字符" name="maskChar" extra="默认 *；取首个字符；保留首尾">
          <Input
            value={maskChar}
            onChange={(e) => setMaskChar(e.target.value)}
            placeholder={DEFAULT_MASK_CHAR}
            allowClear
          />
        </Form.Item>
        <Form.Item>
          <Space>
            <Button type="primary" loading={loading} onClick={handleRun}>
              执行脱敏
            </Button>
            <Button onClick={handleReset}>
              重置
            </Button>
            <Button loading={saving} onClick={handleSaveSetting}>
              保存设置
            </Button>
          </Space>
        </Form.Item>
      </Form>
    </div>
  );
}
