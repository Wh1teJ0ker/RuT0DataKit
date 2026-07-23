import { useReducer, useEffect } from "react";
import { Layout, Card } from "antd";
import { initialState, appReducer } from "./state.js";
import { listBuiltinRules } from "./tauri.js";
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

  // v0.4.4 T11-3：启动时拉取后端内置规则集（builtin_ruleset）写入 state.rules。
  // 内置规则为出厂自带（phone/bankcard/ip 提取等），用户可见可用但不可编辑。
  // 失败时 state.rules 保持初始空集，不阻塞 UI。
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const rs = await listBuiltinRules();
        if (cancelled) return;
        if (rs && Array.isArray(rs.validators)) {
          dispatch({ type: "SET_RULES", rules: rs });
        }
      } catch {
        // 静默：内置规则加载失败不阻塞 UI，state.rules 保持空集。
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

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
