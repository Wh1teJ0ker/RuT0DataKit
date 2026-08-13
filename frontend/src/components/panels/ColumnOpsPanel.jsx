import { useState } from "react";
import { Button, Divider, Form, Select, Typography, message } from "antd";
import { useAppContext } from "../../state";
import { parseColumnAsJson, transformColumn } from "../../tauri";
import { useSheetOps } from "../../hooks/useSheetOps";
import ColumnSelect from "../shared/ColumnSelect";

const { Title } = Typography;

// v1.1.2 列操作面板：
// - JSON 解析为新 Tab：parseColumnAsJson → landNewSheet
// v1.1.2 起 Base64 编解码已迁至独立 CryptoPanel（加解密能力按钮）。
// v1.1.5 T86 新增：列变换（大小写归一化）。
// v1.2.0 T94：op-then-refresh 样板收敛到 useSheetOps，列 Select 收敛到 ColumnSelect。
export default function ColumnOpsPanel() {
  const { state, dispatch, addSheetFromParse } = useAppContext();
  const { refreshActiveSheet, landNewSheet } = useSheetOps(dispatch, addSheetFromParse);
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
      await landNewSheet(res, `${column}_json`, column, sheet.sessionId);
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
      // 刷新当前页数据 + 撤销栈
      await refreshActiveSheet(sheet);
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
    <div style={{ height: "100%", overflow: "auto", padding: 4 }}>
      <Title level={5} style={{ marginTop: 0 }}>
        JSON 解析
      </Title>
      <Form form={parseForm} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
          <ColumnSelect headers={headers} placeholder="选择列" />
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
          <ColumnSelect headers={headers} placeholder="选择列" />
        </Form.Item>
        <Form.Item label="变换操作" name="op">
          <Select
            placeholder="选择操作"
            options={[
              { label: "大写", value: "uppercase" },
              { label: "小写", value: "lowercase" },
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
