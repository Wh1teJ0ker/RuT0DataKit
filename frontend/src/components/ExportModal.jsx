import { useState, useMemo, useEffect } from "react";
import {
  Modal,
  Select,
  Space,
  Typography,
  Checkbox,
  Divider,
  Input,
  Switch,
  Row,
  Col,
  message,
} from "antd";
import {
  exportSheetToCsv,
  exportSheetToJson,
  exportSheetToTxt,
} from "../tauri";

// 导出格式下拉项。CSV/JSON/TXT 可用；XLSX 已移除。
const FORMAT_OPTIONS = [
  { value: "csv", label: "CSV (.csv)" },
  { value: "json", label: "JSON (.json)" },
  { value: "txt", label: "TXT (.txt)" },
];

// 列分隔符下拉项（CSV / TXT 共用）。
const SEPARATOR_OPTIONS = [
  { value: "comma", label: '逗号 ","' },
  { value: "semicolon", label: '分号 ";"' },
  { value: "tab", label: "Tab" },
  { value: "pipe", label: '竖线 "|"' },
  { value: "custom", label: "自定义" },
];

// JSON 缩进下拉项。
const INDENT_OPTIONS = [
  { value: 2, label: "2 空格" },
  { value: 4, label: "4 空格" },
  { value: 0, label: "紧凑（单行）" },
];

// 行尾下拉项（TXT）。
const LINE_ENDING_OPTIONS = [
  { value: "crlf", label: "CRLF (Windows)" },
  { value: "lf", label: "LF (Unix)" },
];

// TXT 导出模式下拉项。
const TXT_MODE_OPTIONS = [
  { value: "merge", label: "合并行（每行数据 → 一行）" },
  { value: "percol", label: "逐列（每列各一行）" },
];

// JSON 结构下拉项。
const JSON_FORMAT_OPTIONS = [
  { value: "array", label: "数组 [{},{}]" },
  { value: "ndjson", label: "NDJSON（每行一对象）" },
];

export default function ExportModal({ open, sheet, onClose }) {
  const [format, setFormat] = useState("csv");
  const [exporting, setExporting] = useState(false);
  // 选中的导出列（header 名数组，顺序遵循 sheet.columnOrder || headers）。
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

  // TXT 默认模板：{字段名}_{值}（字段名与值之间用下划线连接）。
  const defaultTemplate = useMemo(
    () => "{类型}_{数据值}",
    []
  );

  // 每次打开 / 切换 sheet 时重置：列全选 + 各选项回到默认。
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
  }, [open, orderedHeaders, defaultTemplate]);

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
          // 合并行模式：{类型}_{数据值} → ip_163.211.48.156
          // 逐列模式：{字段名}_{值} → 类型_ip（每列一行）
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
          <>
            <Divider style={{ margin: 0 }} />
            <Row gutter={[8, 8]}>
              <Col xs={{ span: 24 }} sm={{ span: 12 }}>
                <Typography.Text>列分隔符：</Typography.Text>
                <Select
                  value={csvSeparator}
                  onChange={setCsvSeparator}
                  style={{ width: "100%", marginTop: 4 }}
                  options={SEPARATOR_OPTIONS}
                />
              </Col>
              <Col xs={{ span: 24 }} sm={{ span: 12 }}>
                <Typography.Text>含表头行：</Typography.Text>
                <div style={{ marginTop: 4 }}>
                  <Switch checked={csvWithHeader} onChange={setCsvWithHeader} />
                </div>
              </Col>
            </Row>
            {csvSeparator === "custom" && (
              <Input
                value={csvCustomSeparator}
                onChange={(e) => setCsvCustomSeparator(e.target.value)}
                placeholder="自定义分隔符，如 | 或 #"
                style={{ marginTop: 4 }}
              />
            )}
          </>
        )}

        {/* JSON 专属选项 */}
        {isJson && orderedHeaders.length > 0 && (
          <>
            <Divider style={{ margin: 0 }} />
            <Row gutter={[8, 8]}>
              <Col xs={{ span: 24 }} sm={{ span: 12 }}>
                <Typography.Text>缩进：</Typography.Text>
                <Select
                  value={jsonIndent}
                  onChange={setJsonIndent}
                  style={{ width: "100%", marginTop: 4 }}
                  options={INDENT_OPTIONS}
                />
              </Col>
              <Col xs={{ span: 24 }} sm={{ span: 12 }}>
                <Typography.Text>结构：</Typography.Text>
                <Select
                  value={jsonFormat}
                  onChange={setJsonFormat}
                  style={{ width: "100%", marginTop: 4 }}
                  options={JSON_FORMAT_OPTIONS}
                />
              </Col>
            </Row>
          </>
        )}

        {/* TXT 专属选项：模式 + 模板 + 行尾 */}
        {isTxt && orderedHeaders.length > 0 && (
          <>
            <Divider style={{ margin: 0 }} />
            <div>
              <Typography.Text>导出模式：</Typography.Text>
              <Select
                value={txtMode}
                onChange={(v) => {
                  setTxtMode(v);
                  if (v === "percol") {
                    setTxtTemplate("{字段名}_{值}");
                  } else {
                    setTxtTemplate("{类型}_{数据值}");
                  }
                }}
                style={{ width: "100%", marginTop: 4 }}
                options={TXT_MODE_OPTIONS}
              />
            </div>
            <div>
              <Typography.Text>TXT 模板：</Typography.Text>
              <Input.TextArea
                value={txtTemplate}
                onChange={(e) => setTxtTemplate(e.target.value)}
                placeholder={txtMode === "merge" ? "{类型}_{数据值}" : "{字段名}_{值}"}
                autoSize={{ minRows: 2, maxRows: 4 }}
                style={{ fontFamily: "monospace", marginTop: 4 }}
              />
              {txtMode === "merge" ? (
                <>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    合并行模式：用 `{"{列名}"}` 直接引用具体列；每行数据只渲染一行，多列值合并到同一行。
                  </Typography.Text>
                  <Typography.Text type="secondary" style={{ fontSize: 12, display: "block" }}>
                    示例：`{"{类型}_{数据值}"}` → ip_163.211.48.156
                  </Typography.Text>
                </>
              ) : (
                <>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    逐列模式：用 `{"{字段名}"}` 引用列名、`{"{值}"}` 引用单元格值；每行数据的每个选中列各渲染一行。
                  </Typography.Text>
                  <Typography.Text type="secondary" style={{ fontSize: 12, display: "block" }}>
                    示例：`{"{字段名}_{值}"}` → 类型_ip（每列一行）
                  </Typography.Text>
                </>
              )}
            </div>
            <Row gutter={[8, 8]}>
              <Col xs={{ span: 24 }} sm={{ span: 12 }}>
                <Typography.Text>行尾：</Typography.Text>
                <Select
                  value={txtLineEnding}
                  onChange={setTxtLineEnding}
                  style={{ width: "100%", marginTop: 4 }}
                  options={LINE_ENDING_OPTIONS}
                />
              </Col>
            </Row>
          </>
        )}

        {/* 通用：列选择（所有格式共用，替代原两个重复块） */}
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
