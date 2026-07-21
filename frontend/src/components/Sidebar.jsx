import { Menu } from "antd";
import {
  AppstoreOutlined,
  SettingOutlined,
  SearchOutlined,
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  DownloadOutlined,
  ToolOutlined,
  ConsoleSqlOutlined,
  CodeOutlined,
} from "@ant-design/icons";

// v0.4.0 Sidebar：7 项 active，按用户主流程排序：
//   数据预处理 → 规则管理 → 搜索 → 数据脱敏 → 数据校验 → 数据导出 → Tools。
// 原 log/pcap 独立项删除（其能力下沉到搜索 / Tools，LogView/PcapView 文件保留供后续复用）。
// 无 disabled 项；点击 dispatch SET_VIEW，未实现的 view 由 App.jsx 渲染占位 Card。
//
// v0.4.1 T6-3：Tools 不再是单个跳转项，而是 SubMenu 下拉——点 Tools 展开两个子项：
//   「SQL 解析」（key=tools.sql，进入 Tools 视图并 SET_TOOLS_ACTIVE_TAB=sql）
//   「正则解析」（key=tools.regex，进入 Tools 视图并 SET_TOOLS_ACTIVE_TAB=regex）
// 点击子项同时 dispatch SET_VIEW + SET_TOOLS_ACTIVE_TAB；Menu openKeys 受控为
// state.sidebarToolsOpen（默认 open=false，保持折叠，点击 Tools 标题才展开）。
export default function Sidebar({ state, dispatch }) {
  const toolsOpen = state.sidebarToolsOpen ?? false;
  const items = [
    { key: "preprocess", icon: <AppstoreOutlined />, label: "数据预处理" },
    { key: "rules", icon: <SettingOutlined />, label: "规则管理" },
    { key: "search", icon: <SearchOutlined />, label: "搜索" },
    { key: "mask", icon: <SafetyCertificateOutlined />, label: "数据脱敏" },
    { key: "validate", icon: <CheckCircleOutlined />, label: "数据校验" },
    { key: "export", icon: <DownloadOutlined />, label: "数据导出" },
    {
      key: "tools",
      icon: <ToolOutlined />,
      label: "Tools",
      children: [
        { key: "tools.sql", icon: <ConsoleSqlOutlined />, label: "SQL 解析" },
        { key: "tools.regex", icon: <CodeOutlined />, label: "正则解析" },
      ],
    },
  ];

  const handleToolsOpenChange = (open) =>
    dispatch({ type: "SET_SIDEBAR_TOOLS_OPEN", sidebarToolsOpen: open });

  const handleClick = ({ key }) => {
    if (key === "tools.sql") {
      dispatch({ type: "SET_VIEW", activeView: "tools" });
      dispatch({ type: "SET_TOOLS_ACTIVE_TAB", toolsActiveTab: "sql" });
    } else if (key === "tools.regex") {
      dispatch({ type: "SET_VIEW", activeView: "tools" });
      dispatch({ type: "SET_TOOLS_ACTIVE_TAB", toolsActiveTab: "regex" });
    } else {
      dispatch({ type: "SET_VIEW", activeView: key });
    }
  };

  // selectedKeys：当前在 tools 视图时高亮 tools.<activeTab>，否则按 activeView 高亮。
  const selectedKeys =
    state.activeView === "tools"
      ? [`tools.${state.toolsActiveTab ?? "sql"}`]
      : [state.activeView];

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
        selectedKeys={selectedKeys}
        openKeys={toolsOpen ? ["tools"] : []}
        onOpenChange={(keys) => {
          // antd inline Menu onOpenChange 给出当前所有展开 SubMenu 的 key 数组；
          // 我们只关心 tools 是否在其中。
          handleToolsOpenChange(keys.includes("tools"));
        }}
        items={items}
        onClick={handleClick}
        style={{ borderInlineEnd: "none", paddingTop: 8 }}
      />
    </div>
  );
}
