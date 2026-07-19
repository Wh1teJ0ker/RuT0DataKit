import { Menu, Typography } from "antd";
import {
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  DownloadOutlined,
  SettingOutlined,
  ProfileOutlined,
  WifiOutlined,
} from "@ant-design/icons";

const { Text } = Typography;

// 6 个 nav item：4 active（mask/validate/export/rules）+ 2 disabled（log v0.1.1 / pcap v0.1.2）。
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
      disabled: true,
      label: (
        <span>
          日志扫描 <Text type="secondary" style={{ fontSize: 11 }}>v0.1.1</Text>
        </span>
      ),
    },
    {
      key: "pcap",
      icon: <WifiOutlined />,
      disabled: true,
      label: (
        <span>
          流量分析 <Text type="secondary" style={{ fontSize: 11 }}>v0.1.2</Text>
        </span>
      ),
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
