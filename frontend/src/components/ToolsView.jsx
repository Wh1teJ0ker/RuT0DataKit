import React from "react";
import { Card, Typography } from "antd";
import RegexTool from "./RegexTool.jsx";
import SqlParseTool from "./SqlParseTool.jsx";

const { Text } = Typography;

// Tools 容器（v0.4.1 T6-3）：Tools 子工具切换入口已上移到 Sidebar SubMenu，
// 点击 Sidebar「Tools → SQL 解析」/「Tools → 正则解析」即同时 dispatch
// SET_VIEW=tools + SET_TOOLS_ACTIVE_TAB=<sql|regex>。本视图不再渲染顶部 Select
// 下拉（与 Sidebar 下拉语义重复），仅渲染对应子工具内容。
//
// toolsActiveTab 由 Sidebar 写入，切 view 不重置；切 Tab 也不清各自子状态。
export default function ToolsView({ state, dispatch }) {
  return (
    <Card
      title={
        state.toolsActiveTab === "regex" ? "Tools - 正则解析" : "Tools - SQL 解析"
      }
      extra={
        <Text type="secondary">
          切换子工具请使用左侧 Sidebar 「Tools」下拉项
        </Text>
      }
      styles={{ body: { padding: 12 } }}
    >
      {state.toolsActiveTab === "regex" ? (
        <RegexTool state={state} dispatch={dispatch} />
      ) : (
        <SqlParseTool state={state} dispatch={dispatch} />
      )}
    </Card>
  );
}
