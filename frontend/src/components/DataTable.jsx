import { useMemo, useState } from "react";
import { Table, Checkbox, Dropdown, Button, Space } from "antd";
import { SettingOutlined } from "@ant-design/icons";
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
  useSortable,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import "./DataTable.css";

// 可拖拽列头单元格（@dnd-kit/sortable）。
// antd Table 通过 components.header.cell 注入；列 key 经 onHeaderCell 以 data-colkey 传入。
function SortableHeaderCell({ colkey, style, ...rest }) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: colkey });

  const composedStyle = {
    ...style,
    transform: CSS.Translate.toString(transform),
    transition,
    cursor: "move",
    opacity: isDragging ? 0.5 : 1,
    background: isDragging ? "#fff" : style?.background,
    touchAction: "none",
  };

  return (
    <th
      ref={setNodeRef}
      style={composedStyle}
      {...attributes}
      {...listeners}
      {...rest}
    />
  );
}

// components.header.cell 包装：带 data-colkey 的走 SortableHeaderCell，否则普通 th。
function HeaderCell({ "data-colkey": colkey, ...rest }) {
  if (colkey) return <SortableHeaderCell colkey={colkey} {...rest} />;
  return <th {...rest} />;
}

// antd Table 封装（T3）。
// - 行复选 + Shift 区间选择：onSelect 自记 lastSelectedIndex，Shift 时选 [last,current] 区间
// - 列 checkbox 显隐：Dropdown + Checkbox 切换 columnVisibility
// - 列拖拽排序：@dnd-kit/sortable（硬需求，不允许降级为仅列宽）
// - 状态高亮：rowClassName 注入 default/invalid/masked/hit；v1.0.0 mock 全 default
// - 分页：Table.pagination，pageSize=50
//
// TODO(T5): replace mock with real import —— mock 数据由 state 注入，T5 接管后由真实导入流填充。
export default function DataTable({
  sheet,
  onSetSelection,
  onReorderColumns,
  onSetColumnVisibility,
  onSetPage,
}) {
  const [colMenuOpen, setColMenuOpen] = useState(false);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

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
        // 通过 onHeaderCell 把列 key 透传给 header cell 组件
        onHeaderCell: () => ({ "data-colkey": h }),
      }));
  }, [sheet]);

  // 行 key → 当前 sheet.rows 索引（Shift 区间选择用）
  const rowKeyIndex = useMemo(() => {
    const map = new Map();
    sheet?.rows?.forEach((r, i) => map.set(r.key, i));
    return map;
  }, [sheet]);

  function handleSelect(record, checked, _selectedRows, e) {
    if (!sheet) return;
    const rows = sheet.rows;
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
    onSetSelection({
      selectedRowKeys: nextKeys,
      lastSelectedIndex: nextLast,
    });
  }

  function handleSelectAll(checked, _, changeRows) {
    if (!sheet) return;
    const prev = sheet.selection.selectedRowKeys || [];
    const changed = changeRows.map((r) => r.key);
    const next = checked
      ? Array.from(new Set([...prev, ...changed]))
      : prev.filter((k) => !changed.includes(k));
    onSetSelection({
      selectedRowKeys: next,
      lastSelectedIndex: sheet.selection.lastSelectedIndex,
    });
  }

  function handleDragEnd(event) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const order = sheet.columnOrder || sheet.headers;
    const oldIdx = order.indexOf(active.id);
    const newIdx = order.indexOf(over.id);
    if (oldIdx < 0 || newIdx < 0) return;
    onReorderColumns(arrayMove(order, oldIdx, newIdx));
  }

  if (!sheet) return null;

  const visibleHeaders = (sheet.columnOrder || sheet.headers).filter(
    (h) => sheet.columnVisibility?.[h] !== false
  );
  const sortableItems = visibleHeaders.map((h) => ({ id: h }));

  const colMenu = {
    items: sheet.headers.map((h) => ({
      key: h,
      label: (
        <Checkbox
          checked={sheet.columnVisibility?.[h] !== false}
          onChange={(e) => onSetColumnVisibility({ [h]: e.target.checked })}
        >
          {h}
        </Checkbox>
      ),
    })),
    onClick: (info) => info.domEvent?.stopPropagation(),
  };

  return (
    <div className="data-table-wrap">
      <div className="data-table-toolbar">
        <Space>
          <Dropdown
            menu={colMenu}
            open={colMenuOpen}
            onOpenChange={(v) => setColMenuOpen(v)}
            trigger={["click"]}
          >
            <Button icon={<SettingOutlined />}>列显隐</Button>
          </Dropdown>
        </Space>
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
            dataSource={sheet.rows}
            pagination={{
              current: sheet.page,
              pageSize: sheet.pageSize || 50,
              total: sheet.total ?? sheet.rows.length,
              onChange: (page) => onSetPage(page),
              showSizeChanger: false,
            }}
            size="small"
            bordered
            scroll={{ x: "max-content" }}
            components={{
              header: { cell: HeaderCell },
            }}
          />
        </SortableContext>
      </DndContext>
    </div>
  );
}
