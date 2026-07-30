import { Button, Layout, Typography } from "antd";
import {
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  RobotOutlined,
} from "@ant-design/icons";

const { Sider } = Layout;

// 右侧 AI 面板占位：v1.1+ 释放，预留 ai_suggest / invoke_ai_op 契约。
// 可折叠：折叠时 collapsed=true，Sider 收为 collapsedWidth 留图标条，确保展开按钮始终可点。
export default function AiPanel({ visible, setVisible }) {
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
        // 折叠态：仅留一个展开按钮的图标条，始终可点。
        <Button
          type="text"
          icon={<MenuUnfoldOutlined />}
          onClick={() => setVisible(true)}
          style={{ width: 48, height: 48 }}
        />
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
              onClick={() => setVisible(false)}
            />
          </div>
          <div style={{ flex: 1, padding: 12, overflow: "auto" }}>
            <Typography.Paragraph type="secondary">
              AI 能力 v1.1+ 释放
            </Typography.Paragraph>
            <Typography.Paragraph type="secondary" style={{ fontSize: 12 }}>
              预留 IPC 契约：<code>ai_suggest</code> /{" "}
              <code>invoke_ai_op</code>
            </Typography.Paragraph>
          </div>
        </div>
      )}
    </Sider>
  );
}
