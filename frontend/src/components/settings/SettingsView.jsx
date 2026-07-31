import { Button, Space } from "antd";
import { ArrowLeftOutlined } from "@ant-design/icons";
import UpdateCard from "./cards/UpdateCard";
import TsharkPathCard from "./cards/TsharkPathCard";
import DbPathCard from "./cards/DbPathCard";
import AboutCard from "./cards/AboutCard";

// 设置页：全屏独立页面（不复用三栏工作台布局）。
// 顶部返回按钮 → 切回 workbench；主体为垂直有序列表，4 张卡片按固定顺序排列。
export default function SettingsView({ onBack }) {
  return (
    <div
      style={{
        maxWidth: 800,
        margin: "0 auto",
        padding: "24px 16px",
        height: "100%",
        overflow: "auto",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          marginBottom: 16,
        }}
      >
        <Button icon={<ArrowLeftOutlined />} onClick={onBack}>
          返回
        </Button>
        <span style={{ fontSize: 18, fontWeight: 600 }}>设置</span>
      </div>
      <Space direction="vertical" size={16} style={{ width: "100%" }}>
        <UpdateCard />
        <TsharkPathCard />
        <DbPathCard />
        <AboutCard />
      </Space>
    </div>
  );
}
