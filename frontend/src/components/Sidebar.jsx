import { Menu } from "antd";
import {
  AppstoreOutlined,
  SettingOutlined,
  SearchOutlined,
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  DownloadOutlined,
  ToolOutlined,
} from "@ant-design/icons";

// v0.4.0 Sidebar：7 项 active，按用户主流程排序：
//   数据预处理 → 规则管理 → 搜索 → 数据脱敏 → 数据校验 → 数据导出 → Tools。
// 原 log/pcap 独立项删除（其能力下沉到搜索 / Tools，LogView/PcapView 文件保留供后续复用）。
// 无 disabled 项；点击 dispatch SET_VIEW，未实现的 view 由 App.jsx 渲染占位 Card。
export default function Sidebar({ state, dispatch }) {
  const items = [
    { key: "preprocess", icon: <AppstoreOutlined />, label: "数据预处理" },
    { key: "rules", icon: <SettingOutlined />, label: "规则管理" },
    { key: "search", icon: <SearchOutlined />, label: "搜索" },
    { key: "mask", icon: <SafetyCertificateOutlined />, label: "数据脱敏" },
    { key: "validate", icon: <CheckCircleOutlined />, label: "数据校验" },
    { key: "export", icon: <DownloadOutlined />, label: "数据导出" },
    { key: "tools", icon: <ToolOutlined />, label: "Tools" },
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
