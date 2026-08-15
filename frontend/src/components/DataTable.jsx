import { useMemo, useState } from "react";
import { Table, Form, message } from "antd";
import {
  DndContext,
  PointerSensor,
  useSensor,
  useSensors,
  closestCenter,
} from "@dnd-kit/core";
import {
  SortableContext,
  horizontalListSortingStrategy,
  arrayMove,
} from "@dnd-kit/sortable";
import { useAppContext, ACTION } from "../state";
import { searchRows, replaceAll } from "../tauri";
import { PAGE_SIZE } from "../constants";
import { useStaleGuard } from "../hooks/useStaleGuard";
import { useSheetOps } from "../hooks/useSheetOps";
import { highlightCell } from "../utils/highlightCell";
import { HeaderCell } from "./DataTable/DndHeader";
import { SearchToolbar } from "./DataTable/SearchToolbar";
import { ReplaceModal } from "./DataTable/ReplaceModal";
import "./DataTable.css";

// antd Table 封装（容器组件，≤300 行）。
// - 行复选 + Shift 区间选择：onSelect 自记 lastSelectedIndex，Shift 时选 [last,current] 区间
// - 列 checkbox 显隐：Dropdown + Checkbox 切换 columnVisibility
// - 列拖拽排序：@dnd-kit/sortable（硬需求，不允许降级为仅列宽）
// - 状态高亮：rowClassName 注入 default/invalid/masked/hit；v1.1.0 ValidatePanel/MaskPanel/ExtractPanel 触发
// - 分页：Table.pagination，pageSize=PAGE_SIZE
// - v1.1.1 搜索栏 + 单元格 <mark> 高亮 + 全局替换 Modal
//
// 搜索栏、替换 Modal 已拆为子组件（SearchToolbar / ReplaceModal）。
// DnD 列头已拆为 DndHeader.jsx。
export default function DataTable({ sheet, onSetPage }) {
  const { state, dispatch } = useAppContext();
  const { run, isStale } = useStaleGuard();
  const { refreshActiveSheet } = useSheetOps(dispatch);

  const [replaceOpen, setReplaceOpen] = useState(false);
  const [searching, setSearching] = useState(false);
  const [replacing, setReplacing] = useState(false);
  const [replaceForm] = Form.useForm();

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const isSearchMode = !!sheet?.searchRows;

  // 按 columnOrder 排序 + 过滤隐藏列 → antd columns
  const columns = useMemo(() => {
    if (!sheet) return [];
    const order = sheet.columnOrder || sheet.headers;
    return order
      .filter((h) => sheet.columnVisibility?.[h] !== false)
      .map((h) => ({
        title: h,
        dataIndex: h,
        key: h,
        onHeaderCell: () => ({ "data-colkey": h }),
        ellipsis: true,
        // v1.1.1 搜索命中高亮：取 sheet.searchHits?.[record.key]?.[h]
        // 兜底截断：highlightCell 返回纯字符串且超 200 字符时截断加 …，
        // 防止 antd ellipsis 在某些场景未生效时整页渲染长文本
        render: (text, record) => {
          const hits = sheet.searchHits?.[record.key]?.[h];
          const rendered = highlightCell(text, hits);
          if (typeof rendered === "string" && rendered.length > 200) {
            return rendered.slice(0, 200) + "…";
          }
          return rendered;
        },
      }));
  }, [sheet]);

  // 行 key → 当前行索引（Shift 区间选择用）。搜索态用 searchRows，普通态用 rows。
  const rowKeyIndex = useMemo(() => {
    const map = new Map();
    const rows = sheet?.searchRows ?? sheet?.rows;
    rows?.forEach((r, i) => map.set(r.key, i));
    return map;
  }, [sheet]);

  function handleSelect(record, checked, _selectedRows, e) {
    if (!sheet) return;
    const rows = sheet.searchRows ?? sheet.rows;
    const currentIdx = rowKeyIndex.get(record.key);
    const prevSelected = sheet.selection.selectedRowKeys || [];
    let nextKeys;
    let nextLast;

    if (e?.shiftKey && sheet.selection.lastSelectedIndex !== null) {
      const lastIdx = sheet.selection.lastSelectedIndex;
      const [from, to] =
        lastIdx <= currentIdx ? [lastIdx, currentIdx] : [currentIdx, lastIdx];
      const range = rows.slice(from, to + 1).map((r) => r.key);
      nextKeys = Array.from(new Set([...prevSelected, ...range]));
      nextLast = currentIdx;
    } else {
      nextKeys = checked
        ? Array.from(new Set([...prevSelected, record.key]))
        : prevSelected.filter((k) => k !== record.key);
      nextLast = currentIdx;
    }
    dispatch({
      type: ACTION.SET_SELECTION,
      payload: { selectedRowKeys: nextKeys, lastSelectedIndex: nextLast },
    });
  }

  function handleSelectAll(checked, _, changeRows) {
    if (!sheet) return;
    const prev = sheet.selection.selectedRowKeys || [];
    const changed = changeRows.map((r) => r.key);
    const next = checked
      ? Array.from(new Set([...prev, ...changed]))
      : prev.filter((k) => !changed.includes(k));
    dispatch({
      type: ACTION.SET_SELECTION,
      payload: {
        selectedRowKeys: next,
        lastSelectedIndex: sheet.selection.lastSelectedIndex,
      },
    });
  }

  function handleDragEnd(event) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const order = sheet.columnOrder || sheet.headers;
    const oldIdx = order.indexOf(active.id);
    const newIdx = order.indexOf(over.id);
    if (oldIdx < 0 || newIdx < 0) return;
    dispatch({ type: ACTION.REORDER_COLUMNS, payload: arrayMove(order, oldIdx, newIdx) });
  }

  // 搜索：调 searchRows 取首页命中行 → dispatch APPLY_SEARCH_ROWS。
  // useStaleGuard：用 generation token 隔离陈旧响应——用户连续点搜索/翻页时旧响应晚到则丢弃。
  async function handleSearch() {
    if (!sheet) return;
    const { query, useRegex, colIdx } = state.searchState;
    if (!query) {
      message.warning("请输入搜索关键字");
      return;
    }
    setSearching(true);
    const gen = run();
    try {
      const res = await searchRows(
        sheet.id, query, useRegex, colIdx, 1,
        sheet.pageSize || PAGE_SIZE
      );
      if (isStale(gen)) return;
      dispatch({ type: ACTION.APPLY_SEARCH_ROWS, payload: { sheetId: sheet.id, rows: res } });
      dispatch({ type: ACTION.SET_SEARCH_STATE, payload: { page: 1 } });
      if ((res.total ?? 0) === 0) message.info("无匹配结果");
    } catch (e) {
      if (isStale(gen)) return;
      // eslint-disable-next-line no-console
      console.error("search_rows failed:", e);
      message.error(`搜索失败：${e}`);
    } finally {
      if (!isStale(gen)) setSearching(false);
    }
  }

  // 搜索态翻页：调 searchRows 取对应页命中行。
  async function handleSearchPageChange(page) {
    if (!sheet) return;
    const { query, useRegex, colIdx } = state.searchState;
    setSearching(true);
    const gen = run();
    try {
      const res = await searchRows(
        sheet.id, query, useRegex, colIdx, page,
        sheet.pageSize || PAGE_SIZE
      );
      if (isStale(gen)) return;
      dispatch({ type: ACTION.APPLY_SEARCH_ROWS, payload: { sheetId: sheet.id, rows: res } });
      dispatch({ type: ACTION.SET_SEARCH_STATE, payload: { page } });
    } catch (e) {
      if (isStale(gen)) return;
      // eslint-disable-next-line no-console
      console.error("search_rows page failed:", e);
      message.error(`搜索翻页失败：${e}`);
    } finally {
      if (!isStale(gen)) setSearching(false);
    }
  }

  // 全局替换：replaceAll → 退出搜索态 + 刷新当前页（数据已变）
  async function handleReplace() {
    if (!sheet) return;
    try {
      const values = await replaceForm.validateFields();
      const { from, to, useRegex } = values;
      if (!from) { message.warning("请输入查找内容"); return; }
      setReplacing(true);
      const res = await replaceAll(sheet.id, from, to || "", useRegex);
      message.success(`替换 ${res.affected ?? 0} 处`);
      setReplaceOpen(false);
      replaceForm.resetFields();
      await refreshActiveSheet(sheet);
      dispatch({ type: ACTION.CLEAR_SEARCH });
    } catch (e) {
      if (e?.errorFields) return;
      // eslint-disable-next-line no-console
      console.error("replace_all failed:", e);
      message.error(`替换失败：${e}`);
    } finally {
      setReplacing(false);
    }
  }

  if (!sheet) return null;

  const visibleHeaders = (sheet.columnOrder || sheet.headers).filter(
    (h) => sheet.columnVisibility?.[h] !== false
  );
  const sortableItems = visibleHeaders.map((h) => ({ id: h }));

  return (
    <div className="data-table-wrap">
      <div className="data-table-toolbar">
        <SearchToolbar
          sheet={sheet}
          state={state}
          dispatch={dispatch}
          searching={searching}
          onSearch={handleSearch}
          onOpenReplace={() => {
            replaceForm.setFieldsValue({ useRegex: state.searchState.useRegex });
            setReplaceOpen(true);
          }}
        />
      </div>
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
      >
        <SortableContext
          items={sortableItems}
          strategy={horizontalListSortingStrategy}
        >
          <Table
            rowSelection={{
              type: "checkbox",
              selectedRowKeys: sheet.selection.selectedRowKeys,
              onSelect: handleSelect,
              onSelectAll: handleSelectAll,
            }}
            rowClassName={(record) => record.status || "default"}
            columns={columns}
            dataSource={isSearchMode ? sheet.searchRows : sheet.rows}
            pagination={{
              current: isSearchMode
                ? state.searchState.page || 1
                : sheet.page,
              pageSize: sheet.pageSize || PAGE_SIZE,
              total: isSearchMode
                ? sheet.searchTotal ?? 0
                : sheet.total ?? sheet.rows.length,
              onChange: (page) =>
                isSearchMode ? handleSearchPageChange(page) : onSetPage(page),
              showSizeChanger: false,
            }}
            size="small"
            bordered
            scroll={{ x: "max-content", y: "calc(100vh - 280px)" }}
            components={{ header: { cell: HeaderCell } }}
          />
        </SortableContext>
      </DndContext>

      <ReplaceModal
        open={replaceOpen}
        form={replaceForm}
        replacing={replacing}
        onOk={handleReplace}
        onCancel={() => setReplaceOpen(false)}
      />
    </div>
  );
}
