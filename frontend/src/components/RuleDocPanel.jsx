import React from "react";
import { Drawer, Descriptions, Typography, Tag, Empty } from "antd";
import { MASKER_DEFS } from "../maskerDefs.js";
import { VALIDATOR_DEFS } from "../validatorDefs.js";

const { Paragraph } = Typography;

// 规则详情抽屉：根据当前选中的 masker / validator 类型展示 description +
// paramDocs + exampleInput / exampleOutput。RulesPanel 每条规则旁的
// InfoCircleOutlined 按钮触发。
export default function RuleDocPanel({ open, ruleType, ruleName, onClose }) {
  const defs = ruleType === "validator" ? VALIDATOR_DEFS : MASKER_DEFS;
  const def = defs.find((d) => d.name === ruleName);

  return (
    <Drawer
      open={open}
      onClose={onClose}
      placement="right"
      width={420}
      title={
        ruleName ? (
          <span>
            <Tag color={ruleType === "validator" ? "gold" : "blue"}>
              {ruleType === "validator" ? "validator" : "masker"}
            </Tag>
            {ruleName} 说明
          </span>
        ) : (
          "规则说明"
        )
      }
    >
      {!def ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="未选择规则或定义不存在"
        />
      ) : (
        <>
          <Paragraph>{def.description}</Paragraph>
          {def.paramDocs && Object.keys(def.paramDocs).length > 0 ? (
            <Descriptions title="参数" column={1} size="small" bordered>
              {Object.entries(def.paramDocs).map(([k, v]) => (
                <Descriptions.Item key={k} label={k}>
                  {v}
                </Descriptions.Item>
              ))}
            </Descriptions>
          ) : (
            <Paragraph type="secondary" style={{ fontSize: 12 }}>
              该规则无参数
            </Paragraph>
          )}
          <Descriptions title="示例" column={1} size="small" bordered>
            <Descriptions.Item label="输入">
              {def.exampleInput}
            </Descriptions.Item>
            <Descriptions.Item label="输出">
              {def.exampleOutput}
            </Descriptions.Item>
          </Descriptions>
        </>
      )}
    </Drawer>
  );
}
