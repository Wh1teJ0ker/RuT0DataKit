import { Button, Layout, Typography } from "antd";
import {
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  RobotOutlined,
  SettingOutlined,
} from "@ant-design/icons";

import { DEV_STATUS } from "../constants";
import { useAppContext } from "../state";

const { Sider } = Layout;

// 右侧 AI 面板：默认折叠为图标条，展开后仅显示一行状态文案。
// 折叠态图标条底部常驻「⚙ 设置」按钮（与左侧 SidePanel 对称入口）。
// T13：visible / setVisible 经 useAppContext 取，仅保留 onSettings 跨组件回调。
export default function AiPanel({ onSettings }) {
  const { state, setAiPanelVisible } = useAppContext();
  const { visible } = state.aiPanel;
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
        // 折叠态：图标条 + 底部设置按钮。
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
          <Button
            type="text"
            icon={<SettingOutlined />}
            onClick={onSettings}
            style={{ width: 48, height: 48 }}
          />
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
            <Button
              type="text"
              size="small"
              icon={<MenuFoldOutlined />}
              onClick={() => setAiPanelVisible(false)}
            />
          </div>
          <div style={{ flex: 1, padding: 12, overflow: "auto" }}>
            <Typography.Paragraph type="secondary">
              AI 能力 {DEV_STATUS}
            </Typography.Paragraph>
          </div>
          <div
            style={{
              borderTop: "1px solid #f0f0f0",
              padding: 8,
            }}
          >
            <Button
              block
              icon={<SettingOutlined />}
              onClick={onSettings}
            >
              设置
            </Button>
          </div>
        </div>
      )}
    </Sider>
  );
}
