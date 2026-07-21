import React, { useMemo, useState } from "react";
import {
  Card,
  Select,
  Table,
  Button,
  Checkbox,
  Space,
  Typography,
  Switch,
  Alert,
  App as AntApp,
} from "antd";
import { ArrowUpOutlined, ArrowDownOutlined, DownloadOutlined } from "@ant-design/icons";
import { saveDialog, exportRecordsCsv, exportRecordsXlsx, exportRecordsJson } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";

const { Text } = Typography;

// 数据导出视图：源数据 Select（脱敏后 / 校验后 / 原始）+ 行过滤 Select（仅校验源）+
// 格式 Select（CSV / XLSX）+ 导出按钮。主体为单一 antd Table：表头内含
// Checkbox（勾选导出列）+ 上下移按钮（调序），单元格直接预览数据。
// 未勾选的列仍渲染单元格（让用户看到取消勾选的效果），导出时只写 exportColumns。
// v0.4.0 T5-13：消费 SearchView 跳转携带的 filteredRowIndices——当源数据为
// raw / records 且开启「仅搜索命中行」开关时，预览与导出都按命中行过滤。
export default function ExportView({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const {
    headers,
    rows,
    maskedRows,
    validateResult,
    rules,
    filePath,
    columnOrder,
    exportColumns,
    exportFormat,
    exportSource,
    validateFilter,
    loading,
    actionHint,
  } = state;

  const sourceOptions = [
    { label: "脱敏后数据", value: "masked", disabled: !maskedRows },
    { label: "校验后数据", value: "validate", disabled: !validateResult },
    { label: "原始数据", value: "raw", disabled: !filePath },
  ];

  const filterOptions = [
    { label: "全部行", value: "all" },
    { label: "仅合法行", value: "valid" },
    { label: "仅非法行", value: "invalid" },
  ];

  const formatOptions = [
    { label: "CSV", value: "csv" },
    { label: "XLSX", value: "xlsx" },
    { label: "JSON", value: "json" },
  ];

  const moveColumn = (from, to) => {
    if (to < 0 || to >= columnOrder.length) return;
    const next = [...columnOrder];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    dispatch({ type: "SET_COLUMN_ORDER", columnOrder: next });
  };

  const toggleColumn = (col, checked) => {
    const next = checked
      ? [...exportColumns, col]
      : exportColumns.filter((c) => c !== col);
    dispatch({ type: "SET_EXPORT_COLUMNS", exportColumns: next });
  };

  // v0.4.0 T5-13：搜索命中行过滤（仅对 raw / records 源生效）。
  const hasFiltered =
    Array.isArray(state.filteredRowIndices) && state.filteredRowIndices.length > 0;
  const [onlyFiltered, setOnlyFiltered] = useState(true); // 跳转过来默认开启

  // 根据当前 exportSource 计算预览源数据行。
  // - raw / records：原始行（"raw" 为旧别名，归一到 "records" 语义）；
  // - masked：脱敏后行；validate：校验结果行（可按 valid/invalid 过滤）。
  // v0.4.0 T5-13：当源为 raw/records 且开启 onlyFiltered 且 hasFiltered 时，
  // 按 filteredRowIndices 过滤。
  const sourceRows = useMemo(
    function computeSourceRows() {
      let out;
      if (exportSource === "raw" || exportSource === "records") out = rows || [];
      else if (exportSource === "masked") out = maskedRows || [];
      else if (exportSource === "validate") {
        if (!validateResult) return [];
        const vrows = validateResult.rows || [];
        if (validateFilter === "all") out = vrows;
        else {
          const matrix = validateResult.valid_matrix || [];
          out = vrows.filter((_, i) => {
            const rowValid = matrix[i] || [];
            const allValid = rowValid.every((v) => v !== false);
            const anyInvalid = rowValid.some((v) => v === false);
            if (validateFilter === "valid") return allValid;
            if (validateFilter === "invalid") return anyInvalid;
            return true;
          });
        }
      } else return [];
      // 搜索命中行过滤只对 raw/records 源生效（masked 行号已变，validate 已有自己的 filter）。
      if (
        onlyFiltered &&
        hasFiltered &&
        (exportSource === "raw" || exportSource === "records")
      ) {
        const set = new Set(state.filteredRowIndices);
        out = out.filter((_, idx) => set.has(idx));
      }
      return out;
    },
    [
      exportSource,
      rows,
      maskedRows,
      validateResult,
      validateFilter,
      onlyFiltered,
      hasFiltered,
      state.filteredRowIndices,
    ]
  );

  // Table dataSource：截断 PREVIEW_ROW_LIMIT 行，按 columnOrder 顺序转对象数组。
  const dataSource = useMemo(
    () =>
      sourceRows.slice(0, PREVIEW_ROW_LIMIT).map((row, idx) => {
        const obj = { key: idx };
        for (const h of columnOrder) {
          const c = headers.indexOf(h);
          obj[h] = c >= 0 ? row[c] : "";
        }
        return obj;
      }),
    [sourceRows, columnOrder, headers]
  );

  // Table columns：遍历 columnOrder，title 内含 Checkbox + 上下移按钮。
  const columns = useMemo(
    () =>
      columnOrder.map((h, idx) => ({
        key: h,
        dataIndex: h,
        ellipsis: true,
        width: 200,
        title: (
          <Space size="small" wrap>
            <Checkbox
              checked={exportColumns.includes(h)}
              onChange={(e) => toggleColumn(h, e.target.checked)}
            >
              {h}
            </Checkbox>
            <Button
              size="small"
              type="text"
              disabled={idx === 0}
              icon={<ArrowUpOutlined />}
              onClick={() => moveColumn(idx, idx - 1)}
            />
            <Button
              size="small"
              type="text"
              disabled={idx === columnOrder.length - 1}
              icon={<ArrowDownOutlined />}
              onClick={() => moveColumn(idx, idx + 1)}
            />
          </Space>
        ),
      })),
    [columnOrder, exportColumns]
  );

  // 计算导出参数：rulesJson + selectedRowIndices。
  // 对「原始数据」源（raw / records）传空规则集避免脱敏；校验源按行过滤筛选行索引。
  // v0.4.0 T5-13：raw/records 源且开启搜索命中行过滤时，selectedRowIndices
  // 取 filteredRowIndices（与后端 selectedRowIndices 语义对齐——传入要保留的行号）。
  const computeExportArgs = () => {
    let rulesJson;
    let selectedRowIndices = null;
    if (exportSource === "raw" || exportSource === "records") {
      rulesJson = JSON.stringify({ maskers: [], validators: [] });
      if (onlyFiltered && hasFiltered) {
        selectedRowIndices = [...state.filteredRowIndices].sort((a, b) => a - b);
      }
    } else {
      rulesJson = JSON.stringify(rules);
    }
    if (exportSource === "validate" && validateResult) {
      const matrix = validateResult.valid_matrix || [];
      const total = (validateResult.rows || []).length;
      const idx = [];
      for (let i = 0; i < total; i++) {
        const rowValid = matrix[i] || [];
        const allValid = rowValid.every((v) => v !== false);
        const anyInvalid = rowValid.some((v) => v === false);
        if (validateFilter === "all") {
          idx.push(i);
        } else if (validateFilter === "valid" && allValid) {
          idx.push(i);
        } else if (validateFilter === "invalid" && anyInvalid) {
          idx.push(i);
        }
      }
      selectedRowIndices = idx;
    }
    return { rulesJson, selectedRowIndices };
  };

  const handleExport = async () => {
    if (!filePath) {
      message.warning("请先导入文件");
      return;
    }
    if (exportColumns.length === 0) {
      message.warning("请至少勾选一列");
      return;
    }
    if (exportSource === "masked" && !maskedRows) {
      message.warning("脱敏结果不存在，请先在脱敏视图应用规则");
      return;
    }
    if (exportSource === "validate" && !validateResult) {
      message.warning("校验结果不存在，请先运行校验");
      return;
    }
    const { rulesJson, selectedRowIndices } = computeExportArgs();
    const ext =
      exportFormat === "xlsx" ? "xlsx" : exportFormat === "json" ? "json" : "csv";
    const defaultName = `export_${Date.now()}.${ext}`;
    const outPath = await saveDialog(defaultName, ext);
    if (!outPath) {
      return;
    }
    dispatch({ type: "SET_LOADING", loading: true });
    dispatch({ type: "SET_HINT", actionHint: "正在导出..." });
    try {
      const args = [
        filePath,
        rulesJson,
        exportColumns,
        columnOrder,
        selectedRowIndices,
        outPath,
      ];
      if (exportFormat === "xlsx") {
        await exportRecordsXlsx(...args);
      } else if (exportFormat === "json") {
        await exportRecordsJson(...args);
      } else {
        await exportRecordsCsv(...args);
      }
      dispatch({ type: "SET_HINT", actionHint: `已导出到 ${outPath}` });
    } catch (e) {
      message.error(`导出失败: ${e}`);
      dispatch({ type: "SET_HINT", actionHint: `导出失败: ${e}` });
    } finally {
      dispatch({ type: "SET_LOADING", loading: false });
    }
  };

  return (
    <Card title="数据导出" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {hasFiltered && (exportSource === "raw" || exportSource === "records") && (
          <Alert
            type="info"
            showIcon
            message={
              <Space size="small">
                <Switch
                  size="small"
                  checked={onlyFiltered}
                  onChange={setOnlyFiltered}
                />
                <Text style={{ fontSize: 13 }}>
                  仅导出搜索命中行（{state.filteredRowIndices.length} 行）
                </Text>
              </Space>
            }
          />
        )}
        <Space size="middle" wrap>
          <span>
            <Text type="secondary" style={{ fontSize: 12, marginRight: 8 }}>
              源数据
            </Text>
            <Select
              value={exportSource}
              options={sourceOptions}
              onChange={(v) =>
                dispatch({ type: "SET_EXPORT_SOURCE", exportSource: v })
              }
              style={{ minWidth: 180 }}
            />
          </span>
          {exportSource === "validate" ? (
            <span>
              <Text type="secondary" style={{ fontSize: 12, marginRight: 8 }}>
                行过滤
              </Text>
              <Select
                value={validateFilter}
                options={filterOptions}
                onChange={(v) =>
                  dispatch({ type: "SET_VALIDATE_FILTER", validateFilter: v })
                }
                style={{ minWidth: 160 }}
              />
            </span>
          ) : null}
          <span>
            <Text type="secondary" style={{ fontSize: 12, marginRight: 8 }}>
              格式
            </Text>
            <Select
              value={exportFormat}
              options={formatOptions}
              onChange={(v) =>
                dispatch({ type: "SET_EXPORT_FORMAT", exportFormat: v })
              }
              style={{ minWidth: 120 }}
            />
          </span>
          <Button
            type="primary"
            icon={<DownloadOutlined />}
            loading={loading}
            onClick={handleExport}
          >
            导出
          </Button>
        </Space>

        <Table
          size="small"
          pagination={false}
          scroll={{ y: 420, x: "max-content" }}
          sticky
          columns={columns}
          dataSource={dataSource}
          locale={{ emptyText: "请先导入文件并选择源数据" }}
        />

        <div>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {actionHint}
          </Text>
        </div>
      </Space>
    </Card>
  );
}
