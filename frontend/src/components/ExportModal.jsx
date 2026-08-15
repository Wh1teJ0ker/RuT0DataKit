import { useState, useMemo, useEffect } from "react";
import {
  Modal,
  Select,
  Space,
  Typography,
  Checkbox,
  Divider,
  message,
} from "antd";
import {
  exportSheetToCsv,
  exportSheetToJson,
  exportSheetToTxt,
} from "../tauri";
import { CsvOptions } from "./export/CsvOptions";
import { JsonOptions } from "./export/JsonOptions";
import { TxtOptions } from "./export/TxtOptions";

// 导出格式下拉项。CSV/JSON/TXT 可用；XLSX 已移除。
const FORMAT_OPTIONS = [
  { value: "csv", label: "CSV (.csv)" },
  { value: "json", label: "JSON (.json)" },
  { value: "txt", label: "TXT (.txt)" },
];

export default function ExportModal({ open, sheet, onClose }) {
  const [format, setFormat] = useState("csv");
  const [exporting, setExporting] = useState(false);
  const [selectedCols, setSelectedCols] = useState([]);

  // ---- 各格式独立选项 ----
  // CSV
  const [csvSeparator, setCsvSeparator] = useState("comma");
  const [csvCustomSeparator, setCsvCustomSeparator] = useState("");
  const [csvWithHeader, setCsvWithHeader] = useState(true);
  // JSON
  const [jsonIndent, setJsonIndent] = useState(2);
  const [jsonFormat, setJsonFormat] = useState("array");
  // TXT
  const [txtTemplate, setTxtTemplate] = useState("{类型}_{数据值}");
  const [txtLineEnding, setTxtLineEnding] = useState("crlf");
  const [txtMode, setTxtMode] = useState("merge");

  // 列顺序：遵循 columnOrder，否则用 headers。
  const orderedHeaders = useMemo(() => {
    if (!sheet) return [];
    return sheet.columnOrder?.length ? sheet.columnOrder : sheet.headers || [];
  }, [sheet]);

  // 每次打开时重置：列全选 + 各选项回到默认。
  useEffect(() => {
    if (open) {
      setSelectedCols(orderedHeaders);
      setCsvSeparator("comma");
      setCsvCustomSeparator("");
      setCsvWithHeader(true);
      setJsonIndent(2);
      setJsonFormat("array");
      setTxtTemplate("{类型}_{数据值}");
      setTxtLineEnding("crlf");
      setTxtMode("merge");
    }
  }, [open, orderedHeaders]);

  const allChecked =
    orderedHeaders.length > 0 && selectedCols.length === orderedHeaders.length;
  const indeterminate =
    selectedCols.length > 0 && selectedCols.length < orderedHeaders.length;

  function toggleCol(h, checked) {
    setSelectedCols((prev) =>
      checked ? [...prev, h] : prev.filter((x) => x !== h)
    );
  }

  function toggleAll(checked) {
    setSelectedCols(checked ? [...orderedHeaders] : []);
  }

  const handleOk = async () => {
    if (!sheet) return;
    if (selectedCols.length === 0) {
      message.warning("请至少选择一列");
      return;
    }
    setExporting(true);
    try {
      let ok = false;
      switch (format) {
        case "csv":
          ok = await exportSheetToCsv(sheet, {
            separator: csvSeparator,
            customSeparator: csvCustomSeparator,
            withHeader: csvWithHeader,
            headers: selectedCols,
          });
          break;
        case "json":
          ok = await exportSheetToJson(sheet, {
            indent: jsonIndent,
            ndjson: jsonFormat === "ndjson",
            headers: selectedCols,
          });
          break;
        case "txt":
          ok = await exportSheetToTxt(sheet, {
            template: txtTemplate.trim() ? txtTemplate : null,
            lineEnding: txtLineEnding,
            headers: selectedCols,
            mode: txtMode,
          });
          break;
        default:
          break;
      }
      if (ok) {
        message.success("导出完成");
        onClose?.();
      }
    } catch (e) {
      message.error(`导出失败：${String(e)}`);
    } finally {
      setExporting(false);
    }
  };

  const isTxt = format === "txt";
  const isCsv = format === "csv";
  const isJson = format === "json";

  return (
    <Modal
      title="导出"
      open={open}
      onOk={handleOk}
      onCancel={onClose}
      okText="导出"
      cancelText="取消"
      confirmLoading={exporting}
      width="90vw"
      style={{ maxWidth: 520 }}
      okButtonProps={{ disabled: !sheet || orderedHeaders.length === 0 }}
    >
      <Space direction="vertical" style={{ width: "100%" }} size="middle">
        {/* 格式选择：下拉列表 */}
        <div>
          <Typography.Text strong>导出格式：</Typography.Text>
          <Select
            value={format}
            onChange={setFormat}
            style={{ width: "100%", marginTop: 6 }}
            options={FORMAT_OPTIONS}
          />
        </div>

        {/* CSV 专属选项 */}
        {isCsv && orderedHeaders.length > 0 && (
          <CsvOptions
            csvSeparator={csvSeparator}
            setCsvSeparator={setCsvSeparator}
            csvCustomSeparator={csvCustomSeparator}
            setCsvCustomSeparator={setCsvCustomSeparator}
            csvWithHeader={csvWithHeader}
            setCsvWithHeader={setCsvWithHeader}
          />
        )}

        {/* JSON 专属选项 */}
        {isJson && orderedHeaders.length > 0 && (
          <JsonOptions
            jsonIndent={jsonIndent}
            setJsonIndent={setJsonIndent}
            jsonFormat={jsonFormat}
            setJsonFormat={setJsonFormat}
          />
        )}

        {/* TXT 专属选项：模式 + 模板 + 行尾 */}
        {isTxt && orderedHeaders.length > 0 && (
          <TxtOptions
            txtTemplate={txtTemplate}
            setTxtTemplate={setTxtTemplate}
            txtLineEnding={txtLineEnding}
            setTxtLineEnding={setTxtLineEnding}
            txtMode={txtMode}
            setTxtMode={setTxtMode}
          />
        )}

        {/* 通用：列选择（所有格式共用） */}
        {orderedHeaders.length > 0 && (
          <>
            <Divider style={{ margin: 0 }} />
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
              }}
            >
              <Typography.Text>选择导出列：</Typography.Text>
              <Checkbox
                checked={allChecked}
                indeterminate={indeterminate}
                onChange={(e) => toggleAll(e.target.checked)}
              >
                全选
              </Checkbox>
            </div>
            <Checkbox.Group value={selectedCols} style={{ width: "100%" }}>
              <div
                style={{
                  maxHeight: "30vh",
                  overflow: "auto",
                  border: "1px solid #f0f0f0",
                  borderRadius: 4,
                  padding: "8px 12px",
                }}
              >
                {orderedHeaders.map((h) => (
                  <div key={h} style={{ lineHeight: "28px" }}>
                    <Checkbox
                      value={h}
                      checked={selectedCols.includes(h)}
                      onChange={(e) => toggleCol(h, e.target.checked)}
                    >
                      {h}
                    </Checkbox>
                  </div>
                ))}
              </div>
            </Checkbox.Group>
            {isTxt && (
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {txtMode === "merge"
                  ? "合并行模式：每行数据渲染一行；模板中 {列名} 引用具体列值。"
                  : "逐列模式：每个选中列各渲染一行；未选中列不导出。"}
              </Typography.Text>
            )}
          </>
        )}
      </Space>
    </Modal>
  );
}
