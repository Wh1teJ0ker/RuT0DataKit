import { useEffect, useState } from "react";
import { Button, Empty, Layout, Space, message } from "antd";
import { UndoOutlined, RedoOutlined } from "@ant-design/icons";
import SheetTabs from "./SheetTabs";
import DataTable from "./DataTable";
import { useAppContext } from "../state";
import {
  getSheetData,
  listUndoableOperations,
  undoOperation,
  redoOperation,
} from "../tauri";
import { PAGE_SIZE } from "../constants";

const { Content } = Layout;

// 中央 Workbench（工作台内容区，不含设置页）。
// - activeSheetId !== null：渲染 SheetTabs + 撤销/重做工具条 + DataTable
// - activeSheetId === null 且 sheets 为空：空态「导入数据后在此展示工作台」
// 设置页已移到 App.jsx 顶层全屏路由，本组件不再处理 currentView。
// T13：sheets / activeSheetId / dispatcher 经子组件各自 useAppContext 取，
// 本组件仅保留 setPage 跨组件回调（App.jsx 注入的翻页拉数据流）。
// v1.1.1：撤销/重做工具条；undoStack 由 useEffect 监听 activeSheetId 加载，
// redoStack 在前端本地维护（T33 未提供 listRedoableOperations）。
export default function Workbench({ setPage }) {
  const { state, dispatch, setUndoStack } = useAppContext();
  const { sheets, activeSheetId, undoStack } = state;
  const [redoStack, setRedoStack] = useState([]);

  const activeSheet =
    sheets?.find((s) => s.id === activeSheetId) || null;

  // 监听 activeSheetId 变化加载 undoStack
  useEffect(() => {
    if (!activeSheetId) {
      setUndoStack([]);
      setRedoStack([]);
      return;
    }
    let alive = true;
    listUndoableOperations(activeSheetId)
      .then((ops) => {
        if (!alive) return;
        setUndoStack(ops || []);
        // 切换 Sheet 时重置 redoStack（redo opId 仅在源 Sheet 有效）
        setRedoStack([]);
      })
      .catch((e) => {
        // eslint-disable-next-line no-console
        console.error("list_undoable_operations failed:", e);
      });
    return () => {
      alive = false;
    };
  }, [activeSheetId, setUndoStack]);

  // 刷新当前页 + 刷新 undoStack（undo/redo 完成后调用）
  async function refreshSheetAndUndo() {
    if (!activeSheet) return;
    const page = activeSheet.page || 1;
    const data = await getSheetData(
      activeSheet.id,
      page,
      activeSheet.pageSize || PAGE_SIZE
    );
    dispatch({
      type: "SET_SHEET_DATA",
      payload: { ...data, sheetId: activeSheet.id },
    });
    try {
      const ops = await listUndoableOperations(activeSheet.id);
      setUndoStack(ops || []);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("list_undoable_operations refresh failed:", e);
    }
  }

  async function handleUndo() {
    if (!activeSheet) return;
    const top = (undoStack || [])[0];
    if (!top) return;
    try {
      await undoOperation(top.id);
      message.success("已撤销");
      setRedoStack((prev) => [...prev, top.id]);
      await refreshSheetAndUndo();
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("undo_operation failed:", e);
      message.error(`撤销失败：${e}`);
    }
  }

  async function handleRedo() {
    if (!activeSheet) return;
    const lastOpId = redoStack[redoStack.length - 1];
    if (lastOpId == null) return;
    try {
      await redoOperation(lastOpId);
      message.success("已重做");
      // 重做后从本地 redoStack 弹出
      setRedoStack((prev) => prev.slice(0, -1));
      await refreshSheetAndUndo();
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("redo_operation failed:", e);
      message.error(`重做失败：${e}`);
    }
  }

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
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              marginBottom: 8,
            }}
          >
            <Space size={4}>
              <Button
                size="small"
                icon={<UndoOutlined />}
                disabled={!undoStack || undoStack.length === 0}
                onClick={handleUndo}
              >
                撤销
              </Button>
              <Button
                size="small"
                icon={<RedoOutlined />}
                disabled={redoStack.length === 0}
                onClick={handleRedo}
              >
                重做
              </Button>
              {undoStack && undoStack.length > 0 && (
                <span style={{ fontSize: 12, color: "#999" }}>
                  可撤销 {undoStack.length}
                </span>
              )}
            </Space>
            <SheetTabs />
          </div>
          <div style={{ flex: 1, minHeight: 0 }}>
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
