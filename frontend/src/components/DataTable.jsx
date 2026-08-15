import { useMemo, useRef, useState } from "react";
import {
  Table,
  Checkbox,
  Dropdown,
  Button,
  Space,
  Input,
  Switch,
  Select,
  Modal,
  Form,
  Typography,
  message,
} from "antd";
import { SettingOutlined, SwapOutlined } from "@ant-design/icons";
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
import { useAppContext } from "../state";
import {
  searchRows,
  replaceAll,
  getSheetData,
  listUndoableOperations,
} from "../tauri";
import { PAGE_SIZE } from "../constants";
import "./DataTable.css";

const { Text } = Typography;

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

// v1.1.1 单元格高亮渲染：
// 取 sheet.searchHits?.[record.key]?.[header]，命中区间用 <mark> 包裹。
// 后端 start/end 为 **字节偏移**（Rust str::find / regex::find 返回字节位置），
// 不能直接用 JS string slice（UTF-16 code unit 索引）切片——中文等多字节字符
// 会导致 end > text.length 误判越界而 break，表现为无高亮。
// 这里用 TextEncoder 取 UTF-8 字节、TextDecoder 按字节区间还原字符串，保证偏移语义一致。
// 单元格文本长度阈值：超过此值时跳过 TextEncoder.encode + mark 高亮，
// 直接返回纯文本，防止 8.5MB 级大单元格在主线程上分配巨型 Uint8Array 冻结 UI。
// 用 text.length（UTF-16 code unit 数）做粗判，避免为判断阈值本身先 encode 一遍。
// 与 ExtractPanel 的阈值保持一致。
const HIGHLIGHT_BYTE_LIMIT = 50000;

const _encoder = new TextEncoder();
const _decoder = new TextDecoder("utf-8", { fatal: false });

function highlightCell(value, hitRanges) {
  if (value == null) return value;
  const text = String(value);
  if (!hitRanges || hitRanges.length === 0) return text;
  // 大单元格防护：跳过 TextEncoder.encode，直接返回纯文本（无 mark 高亮）
  if (text.length > HIGHLIGHT_BYTE_LIMIT) return text;
  const bytes = _encoder.encode(text);
  const byteLen = bytes.length;
  const sorted = [...hitRanges].sort((a, b) => a[0] - b[0]);
  const parts = [];
  let cursor = 0; // 字节游标
  for (const [start, end] of sorted) {
    if (start < cursor) continue; // 越界/重叠，跳过
    if (start > byteLen || end > byteLen) break; // 超出字节长度，终止
    if (start > cursor) {
      parts.push(_decoder.decode(bytes.subarray(cursor, start)));
    }
    parts.push(
      <mark key={`${start}-${end}`} style={{ background: "#fff48f" }}>
        {_decoder.decode(bytes.subarray(start, end))}
      </mark>
    );
    cursor = end;
  }
  if (cursor < byteLen) parts.push(_decoder.decode(bytes.subarray(cursor)));
  return <span>{parts}</span>;
}

// antd Table 封装。
// - 行复选 + Shift 区间选择：onSelect 自记 lastSelectedIndex，Shift 时选 [last,current] 区间
// - 列 checkbox 显隐：Dropdown + Checkbox 切换 columnVisibility
// - 列拖拽排序：@dnd-kit/sortable（硬需求，不允许降级为仅列宽）
// - 状态高亮：rowClassName 注入 default/invalid/masked/hit；v1.1.0 ValidatePanel/MaskPanel/ExtractPanel 触发
// - 分页：Table.pagination，pageSize=PAGE_SIZE
// - v1.1.1 搜索栏（关键字/正则切换 + 列选择）+ 单元格 <mark> 高亮 + 全局替换 Modal
//
// mock 数据由 state 注入；真实导入流由 importFile → getSheetData 填充（reducer SET_SHEET_DATA）。
// selection / columnVisibility / columnOrder 等 dispatcher 经 useAppContext 取；
// sheet 仍由 Workbench 通过 props 注入（当前激活 Sheet 对象）。
export default function DataTable({ sheet, onSetPage }) {
  const {
    state,
    dispatch,
    setSelection,
    reorderColumns,
    setColumnVisibility,
    setSearchState,
    applySearchRows,
    clearSearch,
  } = useAppContext();

  const [colMenuOpen, setColMenuOpen] = useState(false);
  const [replaceOpen, setReplaceOpen] = useState(false);
  const [searching, setSearching] = useState(false);
  const [replacing, setReplacing] = useState(false);
  const [replaceForm] = Form.useForm();

  // T64：搜索请求 generation token。用户快速连续翻页/搜索时，旧响应可能晚于
  // 新响应返回，导致 UI 闪现错误结果。发起前自增，响应返回时若 token 已变则丢弃。
  const searchGenRef = useRef(0);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  // 搜索态：sheet.searchRows !== null 表示当前处于「只保留搜索结果」模式。
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
        // 通过 onHeaderCell 把列 key 透传给 header cell 组件
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
    setSelection({
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
    setSelection({
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
    reorderColumns(arrayMove(order, oldIdx, newIdx));
  }

  // 搜索：调 searchRows 取首页命中行 → dispatch APPLY_SEARCH_ROWS（写入 searchRows +
  // searchTotal + searchHits）。searchRows !== null 即切换为「只保留搜索结果」渲染。
  // T64：用 generation token 隔离陈旧响应——用户连续点搜索/翻页时旧响应晚到则丢弃。
  async function handleSearch() {
    if (!sheet) return;
    const { query, useRegex, colIdx } = state.searchState;
    if (!query) {
      message.warning("请输入搜索关键字");
      return;
    }
    setSearching(true);
    const gen = ++searchGenRef.current;
    try {
      const res = await searchRows(
        sheet.id,
        query,
        useRegex,
        colIdx,
        1,
        sheet.pageSize || PAGE_SIZE
      );
      if (searchGenRef.current !== gen) return;
      applySearchRows({ sheetId: sheet.id, rows: res });
      setSearchState({ page: 1 });
      if ((res.total ?? 0) === 0) {
        message.info("无匹配结果");
      }
    } catch (e) {
      if (searchGenRef.current !== gen) return;
      // eslint-disable-next-line no-console
      console.error("search_rows failed:", e);
      message.error(`搜索失败：${e}`);
    } finally {
      if (searchGenRef.current === gen) setSearching(false);
    }
  }

  // 搜索态翻页：调 searchRows 取对应页命中行。
  // T64：与 handleSearch 共用 searchGenRef，连续翻页时仅最后一次请求生效。
  async function handleSearchPageChange(page) {
    if (!sheet) return;
    const { query, useRegex, colIdx } = state.searchState;
    setSearching(true);
    const gen = ++searchGenRef.current;
    try {
      const res = await searchRows(
        sheet.id,
        query,
        useRegex,
        colIdx,
        page,
        sheet.pageSize || PAGE_SIZE
      );
      if (searchGenRef.current !== gen) return;
      applySearchRows({ sheetId: sheet.id, rows: res });
      setSearchState({ page });
    } catch (e) {
      if (searchGenRef.current !== gen) return;
      // eslint-disable-next-line no-console
      console.error("search_rows page failed:", e);
      message.error(`搜索翻页失败：${e}`);
    } finally {
      if (searchGenRef.current === gen) setSearching(false);
    }
  }

  // 全局替换：replaceAll → 退出搜索态 + 刷新当前页（数据已变）
  async function handleReplace() {
    if (!sheet) return;
    try {
      const values = await replaceForm.validateFields();
      const { from, to, useRegex } = values;
      if (!from) {
        message.warning("请输入查找内容");
        return;
      }
      setReplacing(true);
      const res = await replaceAll(sheet.id, from, to || "", useRegex);
      message.success(`替换 ${res.affected ?? 0} 处`);
      setReplaceOpen(false);
      replaceForm.resetFields();
      // 刷新当前页
      const page = sheet.page || 1;
      const data = await getSheetData(sheet.id, page, sheet.pageSize || PAGE_SIZE);
      dispatch({
        type: "SET_SHEET_DATA",
        payload: { ...data, sheetId: sheet.id },
      });
      // 数据已变 → 退出搜索态（清空 searchRows/searchHits/searchState）
      clearSearch();
      // 刷新撤销栈（全局替换是可撤销操作，不刷新会导致撤销/重做按钮失效）
      try {
        const ops = await listUndoableOperations(sheet.id);
        dispatch({ type: "SET_UNDO_STACK", payload: ops });
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("listUndoableOperations refresh failed:", e);
      }
    } catch (e) {
      if (e?.errorFields) return; // 表单校验失败，antd 自带提示
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

  const colMenu = {
    items: sheet.headers.map((h) => ({
      key: h,
      label: (
        <Checkbox
          checked={sheet.columnVisibility?.[h] !== false}
          onChange={(e) => setColumnVisibility({ [h]: e.target.checked })}
        >
          {h}
        </Checkbox>
      ),
    })),
    onClick: (info) => info.domEvent?.stopPropagation(),
  };

  const colSelectOptions = [
    { label: "全部列", value: null },
    ...sheet.headers.map((h, i) => ({ label: h, value: i })),
  ];

  return (
    <div className="data-table-wrap">
      <div className="data-table-toolbar">
        <Space wrap>
          <Dropdown
            menu={colMenu}
            open={colMenuOpen}
            onOpenChange={(v) => setColMenuOpen(v)}
            trigger={["click"]}
          >
            <Button icon={<SettingOutlined />}>列显隐</Button>
          </Dropdown>
          <Input.Search
            placeholder="搜索内容"
            value={state.searchState.query}
            onChange={(e) => setSearchState({ query: e.target.value })}
            onSearch={handleSearch}
            loading={searching}
            enterButton
            style={{ flex: "1 1 220px", minWidth: 120, maxWidth: 220 }}
          />
          <Space size={4}>
            <Switch
              checked={state.searchState.useRegex}
              onChange={(v) => setSearchState({ useRegex: v })}
              size="small"
            />
            <Text type="secondary" style={{ fontSize: 12 }}>
              正则
            </Text>
          </Space>
          <Select
            value={state.searchState.colIdx}
            onChange={(v) => setSearchState({ colIdx: v })}
            options={colSelectOptions}
            style={{ minWidth: 100, flex: "0 1 140px" }}
            size="small"
          />
          <Button size="small" onClick={clearSearch}>
            清除
          </Button>
          <Button
            size="small"
            icon={<SwapOutlined />}
            onClick={() => {
              replaceForm.setFieldsValue({
                useRegex: state.searchState.useRegex,
              });
              setReplaceOpen(true);
            }}
          >
            全局替换
          </Button>
          {isSearchMode && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              搜索结果共 {sheet.searchTotal ?? 0} 行，仅显示命中行
            </Text>
          )}
        </Space>
        <Text type="secondary" style={{ fontSize: 12, whiteSpace: "nowrap" }}>
          {isSearchMode
            ? `命中 ${sheet.searchTotal ?? 0} / ${sheet.total ?? sheet.rows.length ?? 0} 行`
            : `共 ${sheet.total ?? sheet.rows.length ?? 0} 行`}
        </Text>
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
            components={{
              header: { cell: HeaderCell },
            }}
          />
        </SortableContext>
      </DndContext>

      <Modal
        title="全局替换"
        open={replaceOpen}
        onCancel={() => setReplaceOpen(false)}
        onOk={handleReplace}
        confirmLoading={replacing}
        okText="替换"
        cancelText="取消"
        destroyOnClose
      >
        <Form form={replaceForm} layout="vertical" size="small">
          <Form.Item label="查找" name="from" rules={[{ required: true }]}>
            <Input allowClear autoComplete="off" autoCapitalize="off" spellCheck={false} />
          </Form.Item>
          <Form.Item label="替换为" name="to">
            <Input allowClear autoComplete="off" autoCapitalize="off" spellCheck={false} />
          </Form.Item>
          <Form.Item label="正则" name="useRegex" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
