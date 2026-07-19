import { useReducer } from "react";
import { Layout } from "antd";
import { initialState, appReducer } from "./state.js";
import Sidebar from "./components/Sidebar.jsx";
import FileToolbar from "./components/FileToolbar.jsx";
import MaskView from "./components/MaskView.jsx";
import ValidateView from "./components/ValidateView.jsx";
import ExportView from "./components/ExportView.jsx";
import RulesView from "./components/RulesView.jsx";

const { Sider, Content } = Layout;

export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialState);

  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Sider width={200} theme="light" style={{ borderRight: "1px solid #f0f0f0" }}>
        <Sidebar state={state} dispatch={dispatch} />
      </Sider>
      <Layout>
        <Content style={{ padding: 24, display: "flex", flexDirection: "column", gap: 16 }}>
          {/* 规则管理是独立系统，不显示文件导入工具条；其他视图才显示 */}
          {state.activeView !== "rules" && (
            <FileToolbar state={state} dispatch={dispatch} />
          )}
          {state.activeView === "mask" && <MaskView state={state} dispatch={dispatch} />}
          {state.activeView === "validate" && <ValidateView state={state} dispatch={dispatch} />}
          {state.activeView === "export" && <ExportView state={state} dispatch={dispatch} />}
          {state.activeView === "rules" && <RulesView state={state} dispatch={dispatch} />}
        </Content>
      </Layout>
    </Layout>
  );
}
