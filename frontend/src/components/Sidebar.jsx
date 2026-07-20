import { Menu } from "antd";
import {
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  DownloadOutlined,
  SettingOutlined,
  ProfileOutlined,
  WifiOutlined,
} from "@ant-design/icons";

// 6 个 nav item：6 active（mask/validate/export/rules/log/pcap）。
// 受控：selectedKeys 来自 state.activeView，onClick dispatch SET_VIEW。
export default function Sidebar({ state, dispatch }) {
  const items = [
    { key: "mask", icon: <SafetyCertificateOutlined />, label: "数据脱敏" },
    { key: "validate", icon: <CheckCircleOutlined />, label: "数据校验" },
    { key: "export", icon: <DownloadOutlined />, label: "数据导出" },
    { key: "rules", icon: <SettingOutlined />, label: "规则管理" },
    {
      key: "log",
      icon: <ProfileOutlined />,
      label: "日志扫描",
    },
    {
      key: "pcap",
      icon: <WifiOutlined />,
      label: "流量分析",
    },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        style={{
          padding: "16px",
          fontSize: 16,
          fontWeight: 700,
          borderBottom: "1px solid #f0f0f0",
        }}
      >
        RuT0DataKit
      </div>
      <Menu
        mode="inline"
        selectedKeys={[state.activeView]}
        items={items}
        onClick={({ key }) => dispatch({ type: "SET_VIEW", activeView: key })}
        style={{ borderInlineEnd: "none", paddingTop: 8 }}
      />
    </div>
  );
}
