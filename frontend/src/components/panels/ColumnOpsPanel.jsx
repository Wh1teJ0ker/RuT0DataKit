import { useState } from "react";
import { Button, Form, Select, message } from "antd";
import { useAppContext } from "../../state";
import { getSheetData, parseColumnAsJson } from "../../tauri";
import { PAGE_SIZE } from "../../constants";

// v1.1.2 列操作面板：
// - JSON 解析为新 Tab：parseColumnAsJson → dispatch ADD_SHEET_FROM_PARSE → 拉首页
// v1.1.2 起 Base64 编解码已迁至独立 CryptoPanel（加解密能力按钮）。
// 参考 MaskPanel/ExtractPanel 模式（Form + Select + Button + message +
// 操作后 getSheetData + dispatch SET_SHEET_DATA 刷新）。
export default function ColumnOpsPanel() {
  const { state, dispatch, addSheetFromParse } = useAppContext();
  const [parseForm] = Form.useForm();
  const [parsing, setParsing] = useState(false);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];

  async function handleParse() {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = parseForm.getFieldValue("column");
    if (!column) {
      message.warning("请选择目标列");
      return;
    }
    setParsing(true);
    try {
      const res = await parseColumnAsJson(
        sheet.id,
        column,
        sheet.sessionId
      );
      addSheetFromParse({
        newSheetId: res.newSheetId,
        headers: res.headers,
        rowCount: res.rowCount,
        skipped: res.skipped,
        sessionId: sheet.sessionId,
        column,
        name: `${column}_json`,
      });
      // 拉取新 Sheet 首页数据
      const data = await getSheetData(
        res.newSheetId,
        1,
        PAGE_SIZE
      );
      dispatch({
        type: "SET_SHEET_DATA",
        payload: { ...data, sheetId: res.newSheetId },
      });
      message.success(`解析完成：${res.rowCount} 行，跳过 ${res.skipped ?? 0}`);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("parse_column_as_json failed:", e);
      message.error(`解析失败：${e}`);
    } finally {
      setParsing(false);
    }
  }

  return (
    <div style={{ padding: 4 }}>
      <Form form={parseForm} layout="vertical" size="small">
        <Form.Item label="JSON 解析 — 目标列" name="column">
          <Select
            placeholder="选择要解析为 JSON 的列"
            options={headers.map((h) => ({ label: h, value: h }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        <Form.Item>
          <Button
            type="primary"
            loading={parsing}
            onClick={handleParse}
            block
          >
            解析 JSON 为新 Tab
          </Button>
        </Form.Item>
      </Form>
    </div>
  );
}
