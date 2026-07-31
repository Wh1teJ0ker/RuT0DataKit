import { useState, useMemo, useEffect } from "react";
import {
  Modal,
  Radio,
  Space,
  Typography,
  Checkbox,
  Divider,
  Input,
  message,
} from "antd";
import {
  exportSheetToCsv,
  exportSheetToJson,
  exportSheetToTxt,
} from "../tauri";
import { DEV_STATUS } from "../constants";

// 导出格式弹窗：选择目标格式 + 选择导出列后调用对应导出函数。
// v1.0.0 实现 CSV / JSON / TXT；XLSX 暂示「开发中」。
// TXT 支持模板：`{字段名}` 引用字段值，其它字符按字面输出，一行一条记录。
const FORMATS = [
  { value: "csv", label: "CSV (.csv)", disabled: false },
  { value: "json", label: "JSON (.json)", disabled: false },
  { value: "txt", label: "TXT (模板，一行一条)", disabled: false },
  { value: "xlsx", label: `XLSX (.xlsx) ${DEV_STATUS}`, disabled: true },
];

export default function ExportModal({ open, sheet, onClose }) {
  const [format, setFormat] = useState("csv");
  const [exporting, setExporting] = useState(false);
  // 选中的导出列（header 名数组，顺序遵循 sheet.columnOrder || headers）。
  const [selectedCols, setSelectedCols] = useState([]);
  // TXT 模板：`{字段名}` 引用字段值，其它字符按字面输出。留空 → Tab 分隔全列。
  const [txtTemplate, setTxtTemplate] = useState("");

  // 列顺序：遵循 columnOrder，否则用 headers。
  const orderedHeaders = useMemo(() => {
    if (!sheet) return [];
    return sheet.columnOrder?.length ? sheet.columnOrder : sheet.headers || [];
  }, [sheet]);

  // 默认模板：把所有列用 {col}_{col2} 串起来，便于用户直接编辑。
  const defaultTemplate = useMemo(
    () => orderedHeaders.map((h) => `{${h}}`).join("_"),
    [orderedHeaders]
  );

  // 每次打开 / 切换 sheet 时重置：列全选 + 模板回到默认。
  useEffect(() => {
    if (open) {
      setSelectedCols(orderedHeaders);
      setTxtTemplate(defaultTemplate);
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
    // 只把选中列透传给导出函数：rows 保持原对象，函数按 headers 取值。
    const exportSheet = { ...sheet, headers: selectedCols };
    setExporting(true);
    try {
      let ok = false;
      switch (format) {
        case "csv":
          ok = await exportSheetToCsv(exportSheet);
          break;
        case "json":
          ok = await exportSheetToJson(exportSheet);
          break;
        case "txt":
          // 空模板 → Tab 分隔全列；非空 → 按模板渲染。
          ok = await exportSheetToTxt(
            exportSheet,
            txtTemplate.trim() ? txtTemplate : undefined
          );
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

  return (
    <Modal
      title="导出"
      open={open}
      onOk={handleOk}
      onCancel={onClose}
      okText="导出"
      cancelText="取消"
      confirmLoading={exporting}
      okButtonProps={{ disabled: !sheet || orderedHeaders.length === 0 }}
    >
      <Space direction="vertical" style={{ width: "100%" }}>
        <Typography.Text>选择导出格式：</Typography.Text>
        <Radio.Group
          value={format}
          onChange={(e) => setFormat(e.target.value)}
        >
          <Space direction="vertical">
            {FORMATS.map((f) => (
              <Radio key={f.value} value={f.value} disabled={f.disabled}>
                {f.label}
              </Radio>
            ))}
          </Space>
        </Radio.Group>

        {isTxt && orderedHeaders.length > 0 && (
          <>
            <Divider style={{ margin: "8px 0" }} />
            <Typography.Text>TXT 模板：</Typography.Text>
            <Input.TextArea
              value={txtTemplate}
              onChange={(e) => setTxtTemplate(e.target.value)}
              placeholder={`如：{${orderedHeaders[0]}}_{${orderedHeaders[orderedHeaders.length - 1]}}`}
              autoSize={{ minRows: 2, maxRows: 4 }}
              style={{ fontFamily: "monospace" }}
            />
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              字段名用 `{"{字段名}"}` 引用，其它字符（{_}、- 等）按字面输出；
              留空则按 Tab 分隔全列导出。每行数据按模板渲染为一行。
            </Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              可用字段：{orderedHeaders.map((h) => `{${h}}`).join("  ")}
            </Typography.Text>
          </>
        )}

        {!isTxt && orderedHeaders.length > 0 && (
          <>
            <Divider style={{ margin: "8px 0" }} />
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
            <Checkbox.Group
              value={selectedCols}
              style={{ width: "100%" }}
            >
              <div
                style={{
                  maxHeight: 160,
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
          </>
        )}

        {isTxt && orderedHeaders.length > 0 && (
          <>
            <Divider style={{ margin: "8px 0" }} />
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
              }}
            >
              <Typography.Text>模板可用列：</Typography.Text>
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
                  maxHeight: 120,
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
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              仅选中列的值会在模板中渲染；未选中列视为空。
            </Typography.Text>
          </>
        )}
      </Space>
    </Modal>
  );
}
