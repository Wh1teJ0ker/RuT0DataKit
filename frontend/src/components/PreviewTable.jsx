import React from "react";
import { Card, Table, Checkbox, Typography } from "antd";
import { PREVIEW_ROW_LIMIT } from "../state.js";

const { Text } = Typography;

// 预览表 + 列头部 checkbox 勾选要脱敏的列。
// 从 props 拿 state/dispatch（不再用 rowSelection 行选择）。
// state.rows 形状：Vec<Vec<String>>（与后端 load_preview 一致），本地转对象数组。
export default function PreviewTable({ state, dispatch }) {
  const { headers, rows, rowCount, selectedColumns } = state;

  const dataSource = React.useMemo(() => {
    const display = rows.slice(0, PREVIEW_ROW_LIMIT);
    return display.map((row, idx) => {
      const obj = { key: idx };
      for (let c = 0; c < headers.length; c++) {
        obj[headers[c]] = row[c] != null ? row[c] : "";
      }
      return obj;
    });
  }, [rows, headers]);

  const columns = React.useMemo(() => {
    return headers.map((h) => ({
      title: (
        <Checkbox
          checked={selectedColumns.includes(h)}
          onChange={(e) => {
            const next = e.target.checked
              ? [...selectedColumns, h]
              : selectedColumns.filter((c) => c !== h);
            dispatch({ type: "SET_SELECTED_COLUMNS", selectedColumns: next });
          }}
        >
          {h}
        </Checkbox>
      ),
      dataIndex: h,
      key: h,
      ellipsis: true,
    }));
  }, [headers, selectedColumns, dispatch]);

  const total = rowCount || rows.length;
  const shown = dataSource.length;
  const hint =
    headers.length === 0
      ? "尚未导入文件"
      : total > PREVIEW_ROW_LIMIT
      ? `显示前 ${shown} 行，共 ${total} 行`
      : `共 ${total} 行`;

  return (
    <Card
      title="预览（勾选要脱敏的列）"
      extra={<Text type="secondary" style={{ fontSize: 12 }}>{hint}</Text>}
      bodyStyle={{ padding: 12 }}
    >
      <Table
        size="small"
        columns={columns}
        dataSource={dataSource}
        pagination={false}
        scroll={{ y: 360, x: "max-content" }}
        sticky
        locale={{ emptyText: "导入文件后此处显示预览" }}
      />
    </Card>
  );
}
