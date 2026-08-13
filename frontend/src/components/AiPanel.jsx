import { useEffect } from "react";
import { Button, Layout, Tooltip, Typography } from "antd";
import {
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  PushpinOutlined,
  PushpinFilled,
  RobotOutlined,
} from "@ant-design/icons";

import { DEV_STATUS } from "../constants";
import { useAppContext } from "../state";
import { useBreakpoint } from "../hooks/useBreakpoint";

const { Sider } = Layout;

// 右侧 AI 面板：默认折叠为图标条，展开后仅显示一行状态文案。
// 设置入口已迁移到 TopToolbar 右端（T40）。
// v1.2.0：窄窗口（isCompact）时自动折叠，镜像 SidePanel 模式。
export default function AiPanel() {
  const { state, setAiPanelVisible, setAiPanelPinned } = useAppContext();
  const { visible, pinned } = state.aiPanel;
  const { isCompact } = useBreakpoint();

  // 窄窗口自动折叠 AI 面板，避免挤占 Workbench（pinned 时跳过）。
  useEffect(() => {
    if (isCompact && visible && !pinned) {
      setAiPanelVisible(false);
    }
  }, [isCompact, visible, pinned, setAiPanelVisible]);
  return (
    <Sider
      width={300}
      collapsedWidth={48}
      collapsed={!visible}
      theme="light"
      style={{
        height: "100%",
        borderLeft: "1px solid #f0f0f0",
        overflow: "hidden",
      }}
      trigger={null}
    >
      {!visible ? (
        // 折叠态：图标条。
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
            onClick={() => setAiPanelVisible(true)}
            style={{ width: 48, height: 48 }}
          />
          <div style={{ flex: 1 }} />
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            height: "100%",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "8px 12px",
              borderBottom: "1px solid #f0f0f0",
            }}
          >
            <Typography.Text strong>
              <RobotOutlined /> AI 助手
            </Typography.Text>
            <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
              <Tooltip title={pinned ? "取消固定" : "固定（窄屏不自动折叠）"}>
                <Button
                  type="text"
                  size="small"
                  icon={pinned ? <PushpinFilled /> : <PushpinOutlined />}
                  style={{ color: pinned ? "#1677ff" : undefined }}
                  onClick={() => setAiPanelPinned(!pinned)}
                />
              </Tooltip>
              <Button
                type="text"
                size="small"
                icon={<MenuFoldOutlined />}
                onClick={() => setAiPanelVisible(false)}
              />
            </div>
          </div>
          <div style={{ flex: 1, padding: 12, overflow: "auto" }}>
            <Typography.Paragraph type="secondary">
              AI 能力 {DEV_STATUS}
            </Typography.Paragraph>
          </div>
        </div>
      )}
    </Sider>
  );
}
