import { Button, Card, Input, Space, Typography } from "antd";

const { Text } = Typography;

// 设置页 - tshark 路径占位卡片。
// v1.0.0 不持久化、不检测真实路径（tshark 集成属 v1.3+，见 docs/versions/1.0.0/规划需求.md）。
// 仅提供输入框占位 + 「v1.3+ 生效」提示，路径持久化由 T9+ / v1.3+ 接管。
export default function TsharkPathCard() {
  return (
    <Card title="tshark 路径" size="small">
      <Space direction="vertical" style={{ width: "100%" }}>
        <Input placeholder="/usr/local/bin/tshark" disabled />
        <Text type="secondary">tshark 路径配置 v1.3+ 生效</Text>
        <Button disabled>保存</Button>
      </Space>
    </Card>
  );
}
