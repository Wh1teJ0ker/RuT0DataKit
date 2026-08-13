import { useState } from "react";
import { Button, Divider, Form, Select, Typography, message } from "antd";
import { useAppContext } from "../../state";
import {
  getSheetData,
  listUndoableOperations,
  parseColumnAsJson,
  transformColumn,
} from "../../tauri";
import { PAGE_SIZE } from "../../constants";

const { Title } = Typography;

// v1.1.2 列操作面板：
// - JSON 解析为新 Tab：parseColumnAsJson → dispatch ADD_SHEET_FROM_PARSE → 拉首页
// v1.1.2 起 Base64 编解码已迁至独立 CryptoPanel（加解密能力按钮）。
// v1.1.5 T86 新增：列变换（大小写归一化）。
// 参考 MaskPanel/ExtractPanel/CryptoPanel 模式（Form + Select + Button + message +
// 操作后 getSheetData + dispatch SET_SHEET_DATA 刷新）。
export default function ColumnOpsPanel() {
  const { state, dispatch, addSheetFromParse } = useAppContext();
  const [parseForm] = Form.useForm();
  const [transformForm] = Form.useForm();
  const [parsing, setParsing] = useState(false);
  const [transforming, setTransforming] = useState(false);

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

  // v1.1.5 T86：列变换（大小写归一化，就地变更，可撤销）。
  // 流程：transformColumn → getSheetData + SET_SHEET_DATA 刷新当前页 →
  //       listUndoableOperations + SET_UNDO_STACK 刷新撤销栈 → message.success。
  async function handleTransform() {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = transformForm.getFieldValue("column");
    const op = transformForm.getFieldValue("op");
    if (!column) {
      message.warning("请选择目标列");
      return;
    }
    if (!op) {
      message.warning("请选择变换操作");
      return;
    }
    setTransforming(true);
    try {
      const res = await transformColumn(sheet.id, column, op);
      // 刷新当前页数据
      const page = sheet.page || 1;
      const pageSize = sheet.pageSize || PAGE_SIZE;
      const data = await getSheetData(sheet.id, page, pageSize);
      dispatch({
        type: "SET_SHEET_DATA",
        payload: { ...data, sheetId: sheet.id },
      });
      // 刷新撤销栈
      const ops = await listUndoableOperations(sheet.id);
      dispatch({ type: "SET_UNDO_STACK", payload: ops });
      const label = op === "uppercase" ? "大写" : "小写";
      message.success(`列变换完成（${label}）：${res.affected} 行已更新`);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("transform_column failed:", e);
      message.error(`变换失败：${e}`);
    } finally {
      setTransforming(false);
    }
  }

  return (
    <div style={{ padding: 4 }}>
      <Title level={5} style={{ marginTop: 0 }}>
        JSON 解析
      </Title>
      <Form form={parseForm} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
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
      <Divider style={{ margin: "12px 0" }} />
      <Title level={5}>列变换</Title>
      <Form form={transformForm} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
          <Select
            placeholder="选择要变换的列"
            options={headers.map((h) => ({ label: h, value: h }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        <Form.Item label="变换操作" name="op">
          <Select
            placeholder="选择大小写归一化操作"
            options={[
              { label: "大写（UPPERCASE）", value: "uppercase" },
              { label: "小写（lowercase）", value: "lowercase" },
            ]}
          />
        </Form.Item>
        <Form.Item>
          <Button
            type="primary"
            loading={transforming}
            onClick={handleTransform}
            block
          >
            执行变换
          </Button>
        </Form.Item>
      </Form>
    </div>
  );
}
