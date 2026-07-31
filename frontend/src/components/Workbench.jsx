import { Empty, Layout } from "antd";
import SheetTabs from "./SheetTabs";
import DataTable from "./DataTable";
import { useAppContext } from "../state";

const { Content } = Layout;

// 中央 Workbench（工作台内容区，不含设置页）。
// - activeSheetId !== null：渲染 SheetTabs + DataTable
// - activeSheetId === null 且 sheets 为空：空态「导入数据后在此展示工作台」
// 设置页已移到 App.jsx 顶层全屏路由，本组件不再处理 currentView。
// T13：sheets / activeSheetId / dispatcher 经子组件各自 useAppContext 取，
// 本组件仅保留 setPage 跨组件回调（App.jsx 注入的翻页拉数据流）。
export default function Workbench({ setPage }) {
  const { state } = useAppContext();
  const { sheets, activeSheetId } = state;
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
      {activeSheet ? (
        <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
          <SheetTabs />
          <div style={{ flex: 1, minHeight: 0, marginTop: 8 }}>
            {activeSheet.headers && activeSheet.headers.length > 0 ? (
              <DataTable sheet={activeSheet} onSetPage={setPage} />
            ) : (
              <Empty
                style={{ marginTop: 64 }}
                description="空表：点击「导入」加载文件，或在工作台手动添加数据"
              />
            )}
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
