import { useReducer } from "react";
import { Layout, Card } from "antd";
import { initialState, appReducer } from "./state.js";
import Sidebar from "./components/Sidebar.jsx";
import FileToolbar from "./components/FileToolbar.jsx";
import PreprocessView from "./components/PreprocessView.jsx";
import MaskView from "./components/MaskView.jsx";
import ValidateView from "./components/ValidateView.jsx";
import ExportView from "./components/ExportView.jsx";
import RulesView from "./components/RulesView.jsx";
import ToolsView from "./components/ToolsView.jsx";
import SearchView from "./components/SearchView.jsx";

const { Sider, Content } = Layout;

// v0.4.0：Sidebar 7 项 active（preprocess/rules/search/mask/validate/export/tools）。
// LogView/PcapView 作为独立 view 的渲染分支已移除（文件保留供后续复用）。
// 未实现的 view（search/tools）先渲染占位 Card「建设中 - T5-x 交付」。
// 规则管理 / 数据预处理 / 脱敏 / 校验 自带数据入口（preprocess 提供 records，
// rules 维护规则库，mask/validate 消费 state.records），不显示顶部 FileToolbar；
// export 沿用旧 FileToolbar（SET_FILE 路径，T5-8 统一迁移到 records）。
const NO_TOOLBAR_VIEWS = new Set(["preprocess", "rules", "mask", "validate"]);

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
    <Layout style={{ minHeight: "100vh" }}>
      <Sider width={200} theme="light" style={{ borderRight: "1px solid #f0f0f0" }}>
        <Sidebar state={state} dispatch={dispatch} />
      </Sider>
      <Layout>
        <Content style={{ padding: 24, display: "flex", flexDirection: "column", gap: 16 }}>
          {!NO_TOOLBAR_VIEWS.has(view) && <FileToolbar state={state} dispatch={dispatch} />}
          {view === "preprocess" && <PreprocessView state={state} dispatch={dispatch} />}
          {view === "rules" && <RulesView state={state} dispatch={dispatch} />}
          {view === "search" && <SearchView state={state} dispatch={dispatch} />}
          {view === "mask" && <MaskView state={state} dispatch={dispatch} />}
          {view === "validate" && <ValidateView state={state} dispatch={dispatch} />}
          {view === "export" && <ExportView state={state} dispatch={dispatch} />}
          {view === "tools" && <ToolsView state={state} dispatch={dispatch} />}
        </Content>
      </Layout>
    </Layout>
  );
}
