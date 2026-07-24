import React from "react";
import { Card, Typography } from "antd";
import RegexTool from "./RegexTool.jsx";
import SqlParseTool from "./SqlParseTool.jsx";
import EncryptTool from "./EncryptTool.jsx";

const { Text } = Typography;

// Tools 容器（v0.4.1 T6-3）：Tools 子工具切换入口已上移到 Sidebar SubMenu，
// 点击 Sidebar「Tools → SQL 解析」/「Tools → 正则解析」即同时 dispatch
// SET_VIEW=tools + SET_TOOLS_ACTIVE_TAB=<sql|regex>。本视图不再渲染顶部 Select
// 下拉（与 Sidebar 下拉语义重复），仅渲染对应子工具内容。
//
// v0.6.0 T15-2：新增第三分支「加密/解密」（toolsActiveTab="encrypt"），
// 渲染 <EncryptTool>，title 三元对应「Tools - 加密/解密」。
//
// toolsActiveTab 由 Sidebar 写入，切 view 不重置；切 Tab 也不清各自子状态。
export default function ToolsView({ state, dispatch }) {
  const tab = state.toolsActiveTab;
  const title =
    tab === "encrypt"
      ? "Tools - 加密/解密"
      : tab === "regex"
      ? "Tools - 正则解析"
      : "Tools - SQL 解析";
  return (
    <Card
      title={title}
      extra={
        <Text type="secondary">
          切换子工具请使用左侧 Sidebar 「Tools」下拉项
        </Text>
      }
      styles={{ body: { padding: 12 } }}
    >
      {tab === "regex" ? (
        <RegexTool state={state} dispatch={dispatch} />
      ) : tab === "encrypt" ? (
        <EncryptTool state={state} dispatch={dispatch} />
      ) : (
        <SqlParseTool state={state} dispatch={dispatch} />
      )}
    </Card>
  );
}
