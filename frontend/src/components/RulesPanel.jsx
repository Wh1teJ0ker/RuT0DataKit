import React from "react";
import { Card, List, Button, Tag, Empty, Typography, Space } from "antd";
import { DeleteOutlined, EditOutlined, InfoCircleOutlined } from "@ant-design/icons";

const { Text, Paragraph } = Typography;

// 规则列表：每条规则展示 field/masker/params/description，
// 旁边「详情」按钮（打开 RuleDocPanel 抽屉）+「编辑」按钮（dispatch
// SET_EDITING_MASK_RULE 带 __index）+「删除」按钮。
export default function RulesPanel({ rules, dispatch, onShowDoc }) {
  return (
    <Card
      title="脱敏规则"
      extra={
        <Text type="secondary" style={{ fontSize: 12 }}>
          当前 {rules.length} 条规则
        </Text>
      }
      bodyStyle={{ padding: 12 }}
    >
      {rules.length === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="暂无规则，请在下方添加"
        />
      ) : (
        <List
          dataSource={rules}
          renderItem={(rule, idx) => {
            const paramsStr = rule.params
              ? Object.entries(rule.params)
                  .map(([k, v]) => `${k}=${v == null ? "" : String(v)}`)
                  .join(", ")
              : "";
            return (
              <List.Item>
                <Space size="small" wrap direction="vertical" style={{ flex: 1 }}>
                  <Space size="small" wrap>
                    <Text strong>{rule.field}</Text>
                    <Text type="secondary">→</Text>
                    <Tag color="blue">{rule.masker}</Tag>
                    {paramsStr ? (
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        ({paramsStr})
                      </Text>
                    ) : null}
                  </Space>
                  {rule.description ? (
                    <Paragraph
                      type="secondary"
                      ellipsis={{ rows: 2, expandable: true }}
                      style={{ margin: 0, fontSize: 12 }}
                    >
                      {rule.description}
                    </Paragraph>
                  ) : null}
                </Space>
                <Space size="small">
                  <Button
                    type="text"
                    icon={<InfoCircleOutlined />}
                    onClick={() =>
                      onShowDoc
                        ? onShowDoc("masker", rule.masker)
                        : null
                    }
                  />
                  <Button
                    type="text"
                    icon={<EditOutlined />}
                    onClick={() =>
                      dispatch({
                        type: "SET_EDITING_MASK_RULE",
                        rule: { ...rule, __index: idx },
                      })
                    }
                  />
                  <Button
                    type="text"
                    danger
                    icon={<DeleteOutlined />}
                    onClick={() =>
                      dispatch
                        ? dispatch({ type: "REMOVE_MASK_RULE", index: idx })
                        : null
                    }
                  />
                </Space>
              </List.Item>
            );
          }}
        />
      )}
    </Card>
  );
}
