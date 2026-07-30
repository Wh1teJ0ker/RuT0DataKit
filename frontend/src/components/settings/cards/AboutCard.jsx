import { Card, Space, Typography } from "antd";

const { Text, Link } = Typography;

// 设置页 - 关于卡片。
// v1.0.0 版本号 + GitHub / 文档 / License 链接。
const APP_VERSION = "v1.0.0";

const LINKS = [
  { href: "https://github.com/rut0/ruT0datakit", label: "GitHub" },
  { href: "https://github.com/rut0/ruT0datakit#readme", label: "文档" },
  { href: "https://github.com/rut0/ruT0datakit/blob/main/LICENSE", label: "License" },
];

export default function AboutCard() {
  return (
    <Card title="关于" size="small">
      <Space direction="vertical" style={{ width: "100%" }}>
        <Text>
          ruT0datakit <Text strong>{APP_VERSION}</Text>
        </Text>
        <Space size="middle">
          {LINKS.map((l) => (
            <Link key={l.label} href={l.href} target="_blank">
              {l.label}
            </Link>
          ))}
        </Space>
        <Text type="secondary">
          数据脱敏 / 校验 / 提取工作台（v1.0.0 仅布局壳，业务能力 v1.1+ 释放）
        </Text>
      </Space>
    </Card>
  );
}
