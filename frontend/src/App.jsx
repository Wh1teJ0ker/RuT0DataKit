import { useReducer } from "react";
import { Layout, Card } from "antd";
import { initialState, appReducer } from "./state.js";
import Sidebar from "./components/Sidebar.jsx";
import PreprocessView from "./components/PreprocessView.jsx";
import MaskView from "./components/MaskView.jsx";
import ValidateView from "./components/ValidateView.jsx";
import ExportView from "./components/ExportView.jsx";
import ExtractView from "./components/ExtractView.jsx";
import RulesView from "./components/RulesView.jsx";
import ToolsView from "./components/ToolsView.jsx";
import SearchView from "./components/SearchView.jsx";
import SettingsView from "./components/SettingsView.jsx";

const { Sider, Content } = Layout;

// v0.4.0：Sidebar 7 项 active（preprocess/rules/search/mask/validate/export/tools）。
// 数据导入仅作为 PreprocessView 内置入口存在；其他 view 不再渲染顶部常驻导入条
// （T6-2 移除顶部导入工具条）。各 view 在无 records 时自行渲染 Empty 引导用户
// 回到预处理视图导入文件。

function PlaceholderView({ title, task }) {
  return (
    <Card title={title} styles={{ body: { padding: 24 } }}>
      <div style={{ color: "#8c8c8c" }}>建设中 - {task} 交付</div>
    </Card>
  );
}

export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialState);
  const view = state.activeView;

  return (
    <Layout style={{ height: "100vh", overflow: "hidden" }}>
      <Sider
        width={200}
        theme="light"
        style={{ height: "100vh", position: "sticky", top: 0, borderRight: "1px solid #f0f0f0" }}
      >
        <Sidebar state={state} dispatch={dispatch} />
      </Sider>
      <Layout style={{ height: "100vh", overflow: "hidden" }}>
        <Content
          style={{
            padding: 24,
            height: "100%",
            overflowY: "auto",
            display: "flex",
            flexDirection: "column",
            gap: 16,
          }}
        >
          {view === "preprocess" && <PreprocessView state={state} dispatch={dispatch} />}
          {view === "rules" && <RulesView state={state} dispatch={dispatch} />}
          {view === "search" && <SearchView state={state} dispatch={dispatch} />}
          {view === "mask" && <MaskView state={state} dispatch={dispatch} />}
          {view === "validate" && <ValidateView state={state} dispatch={dispatch} />}
          {view === "export" && <ExportView state={state} dispatch={dispatch} />}
          {view === "extract" && <ExtractView state={state} dispatch={dispatch} />}
          {view === "tools" && <ToolsView state={state} dispatch={dispatch} />}
          {view === "settings" && <SettingsView state={state} dispatch={dispatch} />}
        </Content>
      </Layout>
    </Layout>
  );
}
