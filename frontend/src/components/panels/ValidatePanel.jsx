import { useEffect, useState } from "react";
import { Button, Form, Select, Space, message } from "antd";
import { useAppContext } from "../../state";
import { validateColumn, listRules } from "../../tauri";

// v1.1.0 校验面板：选择列 + 规则 → 校验 → 不通过行高亮 invalid。
export default function ValidatePanel() {
  const { state, applyRowStatuses } = useAppContext();
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

  const handleRun = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = form.getFieldValue("column");
    const ruleId = form.getFieldValue("ruleId");
    if (!column) {
      message.warning("请选择要校验的列");
      return;
    }
    if (!ruleId) {
      message.warning("请选择校验规则");
      return;
    }
    setLoading(true);
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
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 4 }}>
      <Form form={form} layout="vertical" size="small">
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
            <Button type="primary" loading={loading} onClick={handleRun}>
              执行校验
            </Button>
            <Button
              onClick={() => {
                form.resetFields();
              }}
            >
              重置
            </Button>
          </Space>
        </Form.Item>
      </Form>
    </div>
  );
}
