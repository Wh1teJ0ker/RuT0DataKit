import { useCallback, useEffect } from "react";
import { Layout } from "antd";
import TopToolbar from "./components/layout/TopToolbar";
import SidePanel from "./components/layout/SidePanel";
import Workbench from "./components/Workbench";
import AiPanel from "./components/AiPanel";
import SettingsView from "./components/settings/SettingsView";
import RulesPanel from "./components/panels/RulesPanel";
import { AppProvider, useAppContext, ACTION } from "./state";
import { getSheetData, loadPageSize } from "./tauri";
import { PAGE_SIZE } from "./constants";
import { useStaleGuard } from "./hooks/useStaleGuard";
import { useActiveSheet } from "./hooks/useActiveSheet";

const { Header, Content } = Layout;

// T2：四区布局 shell。
// Header=TopToolbar / Sider=SidePanel / Content=Workbench / 右侧 Sider=AiPanel。
// 注意：antd Layout 默认 Header 在顶部，下方用 flex 容器横排 SidePanel + Content + AiPanel。
// T5：TopToolbar 的「导入」按钮接入真实导入流（dialog.open → importFile →
// IMPORT_SUCCESS → getSheetData 首页 → 渲染）；翻页时按需拉取对应页数据。
// T13：顶层包裹 <AppProvider>，state/dispatch/dispatcher 经 Context 下发，
// 子组件（TopToolbar / Workbench / AiPanel 等）改用 useAppContext 取数，消除 prop drilling。
// v1.1.0：`activeCapability === "rules"` 时，规则管理面板占据 Workbench 主区，
// 不再走 260px SidePanel（左侧列表 + 右侧详情两栏布局）。
// v1.1.2：启动时加载全局每页行数（settings.json 持久化），dispatch SET_PAGE_SIZE。
function AppShell() {
  const { state, dispatch } = useAppContext();
  const { run, isStale } = useStaleGuard();
  const { sheet: activeSheet } = useActiveSheet();

  // v1.1.2：启动时加载全局每页行数。
  useEffect(() => {
    (async () => {
      try {
        const saved = await loadPageSize();
        if (saved && saved > 0) {
          dispatch({ type: ACTION.SET_PAGE_SIZE, payload: saved });
        }
      } catch {
        // 静默忽略（开发态无 IPC）
      }
    })();
  }, [dispatch]);

  // 导入成功后：dispatch IMPORT_SUCCESS 填充 Sheet，并拉取首页数据。
  // v1.1.2：首页 pageSize 用全局 state.pageSize。
  // useStaleGuard：导入为新 Sheet，理论上无竞态；仍用 generation token 保护一致。
  const handleImport = useCallback(
    async (payload) => {
      dispatch({ type: ACTION.IMPORT_SUCCESS, payload });
      const gen = run();
      try {
        const data = await getSheetData(payload.sheetId, 1, state.pageSize);
        if (isStale(gen)) return;
        dispatch({
          type: ACTION.SET_SHEET_DATA,
          payload: { ...data, sheetId: payload.sheetId },
        });
      } catch (e) {
        if (isStale(gen)) return;
        // 首页拉取失败：保留 Sheet 占位，由用户翻页重试。
        // eslint-disable-next-line no-console
        console.error("getSheetData page 1 failed:", e);
      }
    },
    [dispatch, state.pageSize, run, isStale]
  );

  // 翻页时按需拉取对应页数据。
  // useStaleGuard：用户快速连续翻页时，旧响应可能晚于新响应返回，导致 UI 显示
  // 错误页数据。用 generation token 隔离：发起前自增，响应返回时若 token
  // 已变（用户又翻页了），静默丢弃，不 dispatch SET_SHEET_DATA。
  const handleSetPage = useCallback(
    async (page) => {
      if (!activeSheet) return;
      dispatch({ type: ACTION.SET_PAGE, payload: page });
      const gen = run();
      try {
        const data = await getSheetData(
          activeSheet.id,
          page,
          activeSheet.pageSize || PAGE_SIZE
        );
        if (isStale(gen)) return;
        dispatch({
          type: ACTION.SET_SHEET_DATA,
          payload: { ...data, sheetId: activeSheet.id },
        });
      } catch (e) {
        if (isStale(gen)) return;
        // eslint-disable-next-line no-console
        console.error("getSheetData page failed:", e);
      }
    },
    [activeSheet, dispatch, run, isStale]
  );

  return (
    <Layout style={{ height: "100dvh", minHeight: "100vh", overflow: "hidden" }}>
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
          <SettingsView onBack={() => dispatch({ type: ACTION.SET_VIEW, payload: "workbench" })} />
        </Content>
      ) : state.activeCapability === "rules" ? (
        <Content style={{ background: "#fff", overflow: "hidden" }}>
          <RulesPanel />
        </Content>
      ) : (
        <Layout style={{ overflow: "hidden", minWidth: 0 }}>
          <SidePanel activeCapability={state.activeCapability} />
          <Workbench setPage={handleSetPage} />
          <AiPanel />
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
