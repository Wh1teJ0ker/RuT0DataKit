import { useState, useMemo } from "react";
import { Tabs, Input } from "antd";
import { PlusOutlined } from "@ant-design/icons";
import { useAppContext, ACTION } from "../state";

// Sheet/Tab 多页组件。
// - 新建：+ 按钮 → 产生新 Sheet（dispatch ADD_SHEET）
// - 切换：点击 Tab → dispatch SET_ACTIVE_SHEET
// - 关闭：Tab 上的 × → dispatch CLOSE_SHEET（reducer 内自动激活相邻 Tab）
// - 重命名：双击 Tab 标题进入 Input 编辑，回车确认（dispatch RENAME_SHEET），Esc/失焦取消
//
// sheets / activeSheetId / dispatcher 经 useAppContext 取，消除 prop drilling。
// 真实导入流由 importFile → getSheetData 填充（reducer SET_SHEET_DATA）。
export default function SheetTabs() {
  const { state, dispatch } = useAppContext();
  const { sheets, activeSheetId } = state;

  // 正在编辑的 Tab：{ id, value }
  const [editing, setEditing] = useState(null);

  const items = useMemo(
    () =>
      sheets.map((s) => {
        const isEditing = editing?.id === s.id;
        const title = isEditing ? (
          <Input
            size="small"
            autoFocus
            value={editing.value}
            onChange={(e) =>
              setEditing((prev) =>
                prev ? { ...prev, value: e.target.value } : prev
              )
            }
            onBlur={() => commitRename(s.id, editing.value, s.name)}
            onPressEnter={() => commitRename(s.id, editing.value, s.name)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setEditing(null);
            }}
            onClick={(e) => e.stopPropagation()}
            style={{ width: "100%", minWidth: 80, margin: "-2px 0" }}
          />
        ) : (
          <span
            onDoubleClick={(e) => {
              e.stopPropagation();
              setEditing({ id: s.id, value: s.name });
            }}
          >
            {s.name}
          </span>
        );
        return {
          key: s.id,
          label: title,
          closable: sheets.length > 1,
        };
      }),
    [sheets, editing]
  );

  function commitRename(id, value, fallback) {
    const next = (value || "").trim();
    setEditing(null);
    if (next && next !== fallback) {
      dispatch({ type: ACTION.RENAME_SHEET, payload: { id, name: next } });
    }
  }

  return (
    <Tabs
      type="editable-card"
      activeKey={activeSheetId || undefined}
      items={items}
      onChange={(key) => dispatch({ type: ACTION.SET_ACTIVE_SHEET, payload: key })}
      onEdit={(targetKey, action) => {
        if (action === "add") dispatch({ type: ACTION.ADD_SHEET });
        else if (action === "remove")
          dispatch({ type: ACTION.CLOSE_SHEET, payload: targetKey });
      }}
      addIcon={
        <span>
          <PlusOutlined /> 新建
        </span>
      }
      size="small"
      tabBarStyle={{ margin: 0 }}
      destroyInactiveTabPane
    />
  );
}
