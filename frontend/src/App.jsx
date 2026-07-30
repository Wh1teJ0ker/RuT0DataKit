import { useCallback } from "react";
import { Layout } from "antd";
import TopToolbar from "./components/layout/TopToolbar";
import SidePanel from "./components/layout/SidePanel";
import Workbench from "./components/Workbench";
import AiPanel from "./components/AiPanel";
import { useAppState, ACTION } from "./state";
import { getSheetData } from "./tauri";

const { Header } = Layout;

const PAGE_SIZE = 50;

// T2: 四区布局 shell。
// Header=TopToolbar / Sider=SidePanel / Content=Workbench / 右侧 Sider=AiPanel。
// 注意：antd Layout 默认 Header 在顶部，下方用 flex 容器横排 SidePanel + Content + AiPanel。
// T5：TopToolbar 的「导入」按钮接入真实导入流（dialog.open → importFile →
// IMPORT_SUCCESS → getSheetData 首页 → 渲染）；翻页时按需拉取对应页数据。
export default function App() {
  const {
    state,
    dispatch,
    setView,
    setActiveCapability,
    setAiPanelVisible,
    addSheet,
    closeSheet,
    setActiveSheet,
    renameSheet,
    setSelection,
    reorderColumns,
    setColumnVisibility,
    setPage,
    importSuccess,
    setSheetData,
  } = useAppState();

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
          onImport={handleImport}
        />
      </Header>
      <Layout style={{ overflow: "hidden" }}>
        <SidePanel
          activeCapability={state.activeCapability}
          setView={setView}
        />
        <Workbench
          currentView={state.currentView}
          sheets={state.sheets}
          activeSheetId={state.activeSheetId}
          addSheet={addSheet}
          setActiveSheet={setActiveSheet}
          closeSheet={closeSheet}
          renameSheet={renameSheet}
          setSelection={setSelection}
          reorderColumns={reorderColumns}
          setColumnVisibility={setColumnVisibility}
          setPage={handleSetPage}
        />
        <AiPanel
          visible={state.aiPanel.visible}
          setVisible={setAiPanelVisible}
        />
      </Layout>
    </Layout>
  );
}
