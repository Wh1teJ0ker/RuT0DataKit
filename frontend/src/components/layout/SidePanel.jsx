import { useEffect } from "react";
import { Button, Empty, Layout, Tooltip } from "antd";
import {
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  PushpinOutlined,
  PushpinFilled,
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  FileSearchOutlined,
  ColumnHeightOutlined,
  KeyOutlined,
} from "@ant-design/icons";
import MaskPanel from "../panels/MaskPanel";
import ValidatePanel from "../panels/ValidatePanel";
import ExtractPanel from "../panels/ExtractPanel";
import ColumnOpsPanel from "../panels/ColumnOpsPanel";
import CryptoPanel from "../panels/CryptoPanel";
import { useAppContext } from "../../state";
import { useBreakpoint } from "../../hooks/useBreakpoint";

const { Sider } = Layout;

// v1.1.0：`rules` 能力改在 Workbench 主区渲染两栏布局，不再走 260px SidePanel。
// v1.1.1：新增 `columnOps` 列操作面板。
// v1.1.2：新增 `crypto` 加解密面板（Base64 从 columnOps 迁入，后续扩展哈希/AES 等）。
// v1.1.4 T67：行级校验从独立能力合并入「校验」模块（ValidatePanel 内 Tabs 双页），
//   移除原 `rowValidate` 面板入口与 RowValidatePanel.jsx。
// v1.2.0 T99：支持折叠态（镜像 AiPanel 模式），窄窗口自动折叠为 48px 图标条。
const PANELS = {
  mask: MaskPanel,
  validate: ValidatePanel,
  extract: ExtractPanel,
  columnOps: ColumnOpsPanel,
  crypto: CryptoPanel,
};

// 折叠态图标映射：每个能力对应一个 antd icon。
const CAPABILITY_ICONS = {
  mask: <SafetyCertificateOutlined />,
  validate: <CheckCircleOutlined />,
  extract: <FileSearchOutlined />,
  columnOps: <ColumnHeightOutlined />,
  crypto: <KeyOutlined />,
};

// 左侧动态能力面板容器：由 activeCapability 驱动切换面板；
// activeCapability === null 显示空态。「⚙ 设置」入口已移至右侧 AiPanel。
// v1.1.0：`rules` 不在此列（改由 App.jsx 路由到主区 RulesPanel）。
// v1.2.0 T99：折叠态下点击图标展开面板。
export default function SidePanel({ activeCapability }) {
  const { state, setSidePanelCollapsed, setSidePanelPinned, setActiveCapability } = useAppContext();
  const { collapsed, pinned } = state.sidePanel;
  const { isCompact } = useBreakpoint();
  const PanelComp = activeCapability ? PANELS[activeCapability] : null;

  // 窄窗口自动折叠（pinned 时跳过，保持展开）。
  useEffect(() => {
    if (isCompact && !collapsed && !pinned) {
      setSidePanelCollapsed(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isCompact]);

  return (
    <Sider
      width={260}
      collapsedWidth={48}
      collapsed={collapsed}
      theme="light"
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        overflow: "hidden",
        borderRight: "1px solid #f0f0f0",
      }}
    >
      {collapsed ? (
        // 折叠态：图标条 + 展开按钮。
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            height: "100%",
          }}
        >
          <Button
            type="text"
            icon={<MenuUnfoldOutlined />}
            onClick={() => setSidePanelCollapsed(false)}
            style={{ width: 48, height: 48 }}
          />
          <div style={{ flex: 1 }} />
        </div>
      ) : (
        <div style={{ flex: 1, minHeight: 0, padding: 8, overflow: "auto" }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "flex-end",
              gap: 2,
              marginBottom: 4,
            }}
          >
            <Tooltip title={pinned ? "取消固定" : "固定（窄屏不自动折叠）"}>
              <Button
                type="text"
                size="small"
                icon={pinned ? <PushpinFilled /> : <PushpinOutlined />}
                style={{ color: pinned ? "#1677ff" : undefined }}
                onClick={() => setSidePanelPinned(!pinned)}
              />
            </Tooltip>
            <Button
              type="text"
              size="small"
              icon={<MenuFoldOutlined />}
              onClick={() => setSidePanelCollapsed(true)}
            />
          </div>
          {PanelComp ? (
            <PanelComp />
          ) : (
            <Empty
              style={{ marginTop: 48 }}
              description="点击上方能力按钮展开面板"
            />
          )}
        </div>
      )}
    </Sider>
  );
}
