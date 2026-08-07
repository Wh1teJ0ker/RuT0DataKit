import { useState } from "react";
import { Button, Form, Input, Select, Space, Switch, message } from "antd";
import { useAppContext } from "../../state";
import {
  parseColumnAsJson,
  replaceInColumn,
  getSheetData,
  listUndoableOperations,
} from "../../tauri";
import { PAGE_SIZE } from "../../constants";

// v1.1.1 列操作面板：
// - JSON 解析为新 Tab：parseColumnAsJson → dispatch ADD_SHEET_FROM_PARSE → 拉首页
// - 列内批量替换：replaceInColumn → 刷新当前页 + 刷新 undoStack
// 参考 MaskPanel/ExtractPanel 模式（Form + Select + Button + message +
// 操作后 getSheetData + dispatch SET_SHEET_DATA 刷新）。
export default function ColumnOpsPanel() {
  const { state, dispatch, addSheetFromParse, setUndoStack } =
    useAppContext();
  const [parseForm] = Form.useForm();
  const [replaceForm] = Form.useForm();
  const [parsing, setParsing] = useState(false);
  const [replacing, setReplacing] = useState(false);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];

  async function refreshSheet() {
    if (!sheet) return;
    const page = sheet.page || 1;
    const data = await getSheetData(sheet.id, page, sheet.pageSize || PAGE_SIZE);
    dispatch({
      type: "SET_SHEET_DATA",
      payload: { ...data, sheetId: sheet.id },
    });
  }

  async function refreshUndoStack() {
    if (!sheet) return;
    try {
      const ops = await listUndoableOperations(sheet.id);
      setUndoStack(ops || []);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("list_undoable_operations refresh failed:", e);
    }
  }

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

  async function handleReplace() {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = replaceForm.getFieldValue("column");
    const from = replaceForm.getFieldValue("from");
    const to = replaceForm.getFieldValue("to");
    const useRegex = replaceForm.getFieldValue("useRegex") || false;
    if (!column) {
      message.warning("请选择目标列");
      return;
    }
    if (!from) {
      message.warning("请输入查找内容");
      return;
    }
    setReplacing(true);
    try {
      const res = await replaceInColumn(
        sheet.id,
        column,
        from,
        to || "",
        useRegex
      );
      message.success(`替换 ${res.affected ?? 0} 处`);
      await refreshSheet();
      await refreshUndoStack();
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("replace_in_column failed:", e);
      message.error(`列内替换失败：${e}`);
    } finally {
      setReplacing(false);
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

      <Form form={replaceForm} layout="vertical" size="small">
        <Form.Item label="列内替换 — 目标列" name="column">
          <Select
            placeholder="选择要替换的列"
            options={headers.map((h) => ({ label: h, value: h }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        <Form.Item label="查找" name="from">
          <Input allowClear />
        </Form.Item>
        <Form.Item label="替换为" name="to">
          <Input allowClear />
        </Form.Item>
        <Form.Item label="正则" name="useRegex" valuePropName="checked">
          <Switch />
        </Form.Item>
        <Form.Item>
          <Space>
            <Button
              type="primary"
              loading={replacing}
              onClick={handleReplace}
            >
              列内替换
            </Button>
            <Button
              onClick={() => {
                replaceForm.resetFields();
              }}
            >
              重置
            </Button>
          </Space>
        </Form.Item>
      </Form>
    </div>
  );
}
