import { Button, Layout, Typography } from "antd";
import {
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  RobotOutlined,
} from "@ant-design/icons";
import { useState } from "react";

import { aiSuggest } from "../tauri";

const { Sider } = Layout;

// 右侧 AI 面板占位：v1.1+ 释放，预留 ai_suggest / invoke_ai_op 契约。
// 可折叠：折叠时 collapsed=true，Sider 收为 collapsedWidth 留图标条，确保展开按钮始终可点。
export default function AiPanel({ visible, setVisible }) {
  // v1.0.0 占位：手动触发 aiSuggest 演示契约，错误时展示 v1.1+ 文案。
  const [status, setStatus] = useState("idle"); // idle | pending | unavailable
  const [detail, setDetail] = useState("");

  const onTestAiSuggest = () => {
    setStatus("pending");
    setDetail("");
    aiSuggest({ sheetId: null, selection: null, prompt: null })
      .then(() => {
        // v1.0.0 永不走到这里：后端永远返回 Err。兜底降级。
        setStatus("unavailable");
        setDetail("AI 能力 v1.1+ 释放");
      })
      .catch((err) => {
        // 预期路径：后端返回 "ai_suggest not implemented until v1.1+"
        // 展示 v1.1+ 文案，UI 不崩溃。
        const msg =
          typeof err === "string"
            ? err
            : err?.message || "AI 能力 v1.1+ 释放";
        setStatus("unavailable");
        setDetail(msg);
      });
  };

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
            <div style={{ marginTop: 8 }}>
              <Button
                size="small"
                loading={status === "pending"}
                onClick={onTestAiSuggest}
              >
                调用 ai_suggest 测试
              </Button>
            </div>
            {status === "unavailable" && (
              <Typography.Paragraph
                type="secondary"
                style={{ marginTop: 8, fontSize: 12 }}
              >
                {detail}
              </Typography.Paragraph>
            )}
          </div>
        </div>
      )}
    </Sider>
  );
}
