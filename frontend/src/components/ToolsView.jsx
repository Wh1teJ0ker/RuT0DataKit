import React from "react";
import { Card, Tabs } from "antd";
import RegexTool from "./RegexTool.jsx";
import SqlParseTool from "./SqlParseTool.jsx";

// Tools 容器（v0.4.0）：顶部 Tabs 切换「SQL 解析」「正则解析」两个子工具。
// SQL 解析 Tab 由 T5-10 交付，渲染 <SqlParseTool/>；
// 正则解析 Tab 由 T5-12 交付，渲染 <RegexTool/>。
//
// 顶部 Tab 选中项写入 state.toolsActiveTab，切 view 不重置；切 Tab 也不清
// 各子工具自身状态（子组件状态各自存在 state 里）。
export default function ToolsView({ state, dispatch }) {
  return (
    <Card title="Tools" styles={{ body: { padding: 12 } }}>
      <Tabs
        activeKey={state.toolsActiveTab}
        onChange={(k) =>
          dispatch({ type: "SET_TOOLS_ACTIVE_TAB", toolsActiveTab: k })
        }
        items={[
          {
            key: "sql",
            label: "SQL 解析",
            children: <SqlParseTool state={state} dispatch={dispatch} />,
          },
          {
            key: "regex",
            label: "正则解析",
            children: <RegexTool state={state} dispatch={dispatch} />,
          },
        ]}
      />
    </Card>
  );
}
