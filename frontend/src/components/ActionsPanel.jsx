import React from "react";
import { Card, Space, Button, Typography, App as AntApp } from "antd";
import { PlayCircleOutlined } from "@ant-design/icons";
import { applyRulesCols } from "../tauri.js";

const { Text } = Typography;

// 操作面板：调 applyRulesCols(filePath, rulesJson, selectedColumns)，
// 返回 { masked_rows, headers, summary, skipped_fields } → dispatch SET_MASKED。
// 按钮文案「应用规则到勾选列」。
export default function ActionsPanel({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const { filePath, rules, selectedColumns, loading, actionHint } = state;

  const applyDisabled =
    !filePath || selectedColumns.length === 0 || loading;

  const onApply = async () => {
    if (!filePath) {
      message.warning("请先导入文件");
      return;
    }
    if (selectedColumns.length === 0) {
      message.warning("请至少勾选一列");
      return;
    }
    dispatch({ type: "SET_LOADING", loading: true });
    dispatch({ type: "SET_HINT", actionHint: "正在应用规则..." });
    try {
      const rulesJson = JSON.stringify(rules);
      const r = await applyRulesCols(filePath, rulesJson, selectedColumns);
      dispatch({
        type: "SET_MASKED",
        maskedRows: r.masked_rows,
        maskedSummary: r.summary,
      });
      dispatch({
        type: "SET_HINT",
        actionHint:
          r.summary && r.summary.total != null
            ? `已应用规则，共 ${r.summary.total} 行`
            : "已应用规则到勾选列",
      });
    } catch (e) {
      message.error(`应用失败: ${e}`);
      dispatch({ type: "SET_HINT", actionHint: `应用失败: ${e}` });
    } finally {
      dispatch({ type: "SET_LOADING", loading: false });
    }
  };

  return (
    <Card title="操作" bodyStyle={{ padding: 12 }}>
      <Space size="middle" wrap style={{ marginBottom: 8 }}>
        <Button
          type="primary"
          icon={<PlayCircleOutlined />}
          loading={loading}
          disabled={applyDisabled}
          onClick={onApply}
        >
          应用规则到勾选列
        </Button>
      </Space>
      <div>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {actionHint}
        </Text>
      </div>
    </Card>
  );
}
