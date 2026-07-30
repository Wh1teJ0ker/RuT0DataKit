import { Layout } from "antd";
import TopToolbar from "./components/layout/TopToolbar";
import SidePanel from "./components/layout/SidePanel";
import Workbench from "./components/Workbench";
import AiPanel from "./components/AiPanel";
import { useAppState } from "./state";

const { Header } = Layout;

// T2: 四区布局 shell。
// Header=TopToolbar / Sider=SidePanel / Content=Workbench / 右侧 Sider=AiPanel。
// 注意：antd Layout 默认 Header 在顶部，下方用 flex 容器横排 SidePanel + Content + AiPanel。
export default function App() {
  const { state, setView, setActiveCapability, setAiPanelVisible } =
    useAppState();

  return (
    <Layout style={{ height: "100vh", overflow: "hidden" }}>
      <Header
        style={{
          height: 48,
          lineHeight: "48px",
          padding: 0,
          background: "#fff",
          borderBottom: "1px solid #f0f0f0",
        }}
      >
        <TopToolbar
          activeCapability={state.activeCapability}
          setActiveCapability={setActiveCapability}
        />
      </Header>
      <Layout style={{ overflow: "hidden" }}>
        <SidePanel
          activeCapability={state.activeCapability}
          setView={setView}
        />
        <Workbench currentView={state.currentView} />
        <AiPanel
          visible={state.aiPanel.visible}
          setVisible={setAiPanelVisible}
        />
      </Layout>
    </Layout>
  );
}
