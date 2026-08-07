import { useEffect, useMemo, useState } from "react";
import { Button, Form, List, Select, Space, Tag, Typography, message } from "antd";
import { useAppContext } from "../../state";
import { listRules } from "../../tauri";

const { Text } = Typography;

// v1.1.0 提取面板：选择列 + 多选提取规则 → 按规则 pattern 对当前页数据
// 做正则提取 → 命中行高亮 hit + 命中列表。前端纯逻辑，不写 DB。
export default function ExtractPanel() {
  const { state, applyRowStatuses } = useAppContext();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [rules, setRules] = useState([]);
  const [hits, setHits] = useState([]);

  useEffect(() => {
    let alive = true;
    listRules()
      .then((all) => {
        if (!alive) return;
        setRules(all.filter((r) => r.kind === "extract" && r.pattern));
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
  const rows = sheet?.rows || [];

  const ruleById = useMemo(
    () => Object.fromEntries(rules.map((r) => [r.id, r])),
    [rules]
  );

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

  return (
    <div style={{ padding: 4 }}>
      <Form form={form} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
          <Select
            placeholder="选择要提取的列"
            options={headers.map((h) => ({ label: h, value: h }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        <Form.Item label="提取规则（可多选）" name="ruleIds">
          <Select
            mode="multiple"
            placeholder="选择一条或多条提取规则"
            options={rules.map((r) => ({ label: r.name, value: r.id }))}
            notFoundContent="无可用规则"
          />
        </Form.Item>
        <Form.Item>
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
            style={{ marginTop: 4, maxHeight: 240, overflow: "auto" }}
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
