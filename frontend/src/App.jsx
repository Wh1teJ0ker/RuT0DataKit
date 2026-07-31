import { useCallback } from "react";
import { Layout } from "antd";
import TopToolbar from "./components/layout/TopToolbar";
import SidePanel from "./components/layout/SidePanel";
import Workbench from "./components/Workbench";
import AiPanel from "./components/AiPanel";
import SettingsView from "./components/settings/SettingsView";
import { AppProvider, useAppContext, ACTION } from "./state";
import { getSheetData } from "./tauri";

const { Header, Content } = Layout;

const PAGE_SIZE = 50;

// T2: 四区布局 shell。
// Header=TopToolbar / Sider=SidePanel / Content=Workbench / 右侧 Sider=AiPanel。
// 注意：antd Layout 默认 Header 在顶部，下方用 flex 容器横排 SidePanel + Content + AiPanel。
// T5：TopToolbar 的「导入」按钮接入真实导入流（dialog.open → importFile →
// IMPORT_SUCCESS → getSheetData 首页 → 渲染）；翻页时按需拉取对应页数据。
// T13：顶层包裹 <AppProvider>，state/dispatch/dispatcher 经 Context 下发，
// 子组件（TopToolbar / Workbench / AiPanel 等）改用 useAppContext 取数，消除 prop drilling。
function AppShell() {
  const { state, dispatch, setView, setAiPanelVisible, setPage } =
    useAppContext();

  // 导入成功后：dispatch IMPORT_SUCCESS 填充 Sheet，并拉取首页数据。
  const handleImport = useCallback(
    async (payload) => {
      dispatch({ type: ACTION.IMPORT_SUCCESS, payload });
      try {
        const data = await getSheetData(payload.sheetId, 1, PAGE_SIZE);
        dispatch({
          type: ACTION.SET_SHEET_DATA,
          payload: { ...data, sheetId: payload.sheetId },
        });
      } catch (e) {
        // 首页拉取失败：保留 Sheet 占位，由用户翻页重试。
        // eslint-disable-next-line no-console
        console.error("getSheetData page 1 failed:", e);
      }
    },
    [dispatch]
  );

  // 翻页时按需拉取对应页数据。
  const handleSetPage = useCallback(
    async (page) => {
      const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
      if (!sheet) return;
      setPage(page);
      try {
        const data = await getSheetData(
          sheet.id,
          page,
          sheet.pageSize || PAGE_SIZE
        );
        dispatch({
          type: ACTION.SET_SHEET_DATA,
          payload: { ...data, sheetId: sheet.id },
        });
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("getSheetData page failed:", e);
      }
    },
    [state.sheets, state.activeSheetId, dispatch, setPage]
  );

  // 当前激活的 Sheet（导出按钮据此判断可用性）。
  const activeSheet = state.sheets.find(
    (s) => s.id === state.activeSheetId
  );

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
        <TopToolbar onImport={handleImport} />
      </Header>
      {state.currentView === "settings" ? (
        <Content style={{ background: "#fff", overflow: "auto" }}>
          <SettingsView onBack={() => setView("workbench")} />
        </Content>
      ) : (
        <Layout style={{ overflow: "hidden" }}>
          <SidePanel activeCapability={state.activeCapability} />
          <Workbench setPage={handleSetPage} />
          <AiPanel onSettings={() => setView("settings")} />
        </Layout>
      )}
    </Layout>
  );
}

export default function App() {
  return (
    <AppProvider>
      <AppShell />
    </AppProvider>
  );
}
