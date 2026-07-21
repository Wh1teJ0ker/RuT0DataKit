import React from "react";
import { Card, Select, Typography } from "antd";
import RegexTool from "./RegexTool.jsx";
import SqlParseTool from "./SqlParseTool.jsx";

const { Text } = Typography;

// Tools 容器（v0.4.1）：顶部下拉栏（antd Select）切换「SQL 解析」「正则解析」
// 两个子工具。SQL 解析由 T5-10 交付，渲染 <SqlParseTool/>；正则解析由 T5-12
// 交付，渲染 <RegexTool/>。
//
// 下拉选中项写入 state.toolsActiveTab，切 view 不重置；切下拉也不清各子工具
// 自身状态（子组件状态各自存在 state 里）。
export default function ToolsView({ state, dispatch }) {
  return (
    <Card title="Tools" styles={{ body: { padding: 12 } }}>
      <div>
        <Text type="secondary">选择工具：</Text>{" "}
        <Select
          value={state.toolsActiveTab}
          onChange={(k) =>
            dispatch({ type: "SET_TOOLS_ACTIVE_TAB", toolsActiveTab: k })
          }
          options={[
            { value: "sql", label: "SQL 解析" },
            { value: "regex", label: "正则解析" },
          ]}
          style={{ width: 220 }}
        />
      </div>
      <div style={{ marginTop: 16 }}>
        {state.toolsActiveTab === "regex" ? (
          <RegexTool state={state} dispatch={dispatch} />
        ) : (
          <SqlParseTool state={state} dispatch={dispatch} />
        )}
      </div>
    </Card>
  );
}
