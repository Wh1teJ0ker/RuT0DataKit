import { Button, Input, Select, Space, Switch, Typography, Dropdown, Checkbox, message } from "antd";
import { SettingOutlined, SwapOutlined } from "@ant-design/icons";
import { ACTION } from "../../state";

const { Text } = Typography;

// 搜索栏 + 列显隐 + 全局替换入口（纯展示组件，搜索/翻页/替换逻辑由 DataTable 通过 props 注入）。
// v1.1.1 搜索栏（关键字/正则切换 + 列选择）+ 单元格高亮。
export function SearchToolbar({
  sheet,
  state,
  dispatch,
  searching,
  onSearch,
  onOpenReplace,
}) {
  const isSearchMode = !!sheet?.searchRows;

  const colSelectOptions = [
    { label: "全部列", value: null },
    ...(sheet?.headers || []).map((h, i) => ({ label: h, value: i })),
  ];

  const colMenu = {
    items: sheet?.headers.map((h) => ({
      key: h,
      label: (
        <Checkbox
          checked={sheet.columnVisibility?.[h] !== false}
          onChange={(e) => dispatch({ type: ACTION.SET_COLUMN_VISIBILITY, payload: { [h]: e.target.checked } })}
        >
          {h}
        </Checkbox>
      ),
    })),
    onClick: (info) => info.domEvent?.stopPropagation(),
  };

  return (
    <>
      <Space wrap>
        <Dropdown menu={colMenu} trigger={["click"]}>
          <Button icon={<SettingOutlined />}>列显隐</Button>
        </Dropdown>
        <Input.Search
          placeholder="搜索内容"
          value={state.searchState.query}
          onChange={(e) => dispatch({ type: ACTION.SET_SEARCH_STATE, payload: { query: e.target.value } })}
          onSearch={onSearch}
          loading={searching}
          enterButton
          style={{ flex: "1 1 220px", minWidth: 120, maxWidth: 220 }}
        />
        <Space size={4}>
          <Switch
            checked={state.searchState.useRegex}
            onChange={(v) => dispatch({ type: ACTION.SET_SEARCH_STATE, payload: { useRegex: v } })}
            size="small"
          />
          <Text type="secondary" style={{ fontSize: 12 }}>
            正则
          </Text>
        </Space>
        <Select
          value={state.searchState.colIdx}
          onChange={(v) => dispatch({ type: ACTION.SET_SEARCH_STATE, payload: { colIdx: v } })}
          options={colSelectOptions}
          style={{ minWidth: 100, flex: "0 1 140px" }}
          size="small"
        />
        <Button size="small" onClick={() => dispatch({ type: ACTION.CLEAR_SEARCH })}>
          清除
        </Button>
        <Button
          size="small"
          icon={<SwapOutlined />}
          onClick={onOpenReplace}
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
          ? `命中 ${sheet.searchTotal ?? 0} / ${sheet.total ?? sheet.rows?.length ?? 0} 行`
          : `共 ${sheet.total ?? sheet.rows?.length ?? 0} 行`}
      </Text>
    </>
  );
}
