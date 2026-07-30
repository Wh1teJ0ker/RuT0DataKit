import { Empty, Layout, Typography } from "antd";
import SheetTabs from "./SheetTabs";
import DataTable from "./DataTable";

const { Content } = Layout;

// 中央 Workbench（T2 占位 + T3 扩展）。
// - currentView === 'settings'：T8 设置页覆盖，本任务仅保留占位
// - activeSheetId !== null：渲染 SheetTabs + DataTable
// - activeSheetId === null 且 sheets 为空：T2 空态「导入数据后在此展示工作台」
export default function Workbench({
  currentView,
  sheets,
  activeSheetId,
  addSheet,
  setActiveSheet,
  closeSheet,
  renameSheet,
  setSelection,
  reorderColumns,
  setColumnVisibility,
  setPage,
}) {
  const activeSheet =
    sheets?.find((s) => s.id === activeSheetId) || null;

  return (
    <Content
      style={{
        flex: 1,
        minWidth: 0,
        padding: 24,
        background: "#fff",
        overflow: "auto",
      }}
    >
      {currentView === "settings" ? (
        <Typography.Text type="secondary">
          设置页 v1.0.0 T8 释放
        </Typography.Text>
      ) : activeSheet ? (
        <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
          <SheetTabs
            sheets={sheets}
            activeSheetId={activeSheetId}
            onAdd={() => addSheet()}
            onActive={setActiveSheet}
            onClose={closeSheet}
            onRename={renameSheet}
          />
          <div style={{ flex: 1, minHeight: 0, marginTop: 8 }}>
            <DataTable
              sheet={activeSheet}
              onSetSelection={setSelection}
              onReorderColumns={reorderColumns}
              onSetColumnVisibility={setColumnVisibility}
              onSetPage={setPage}
            />
          </div>
        </div>
      ) : (
        <Empty
          style={{ marginTop: 64 }}
          description="导入数据后在此展示工作台"
        />
      )}
    </Content>
  );
}
