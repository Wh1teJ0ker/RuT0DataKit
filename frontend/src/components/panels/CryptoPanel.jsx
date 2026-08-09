import { useState } from "react";
import { Button, Form, Select, Typography, message } from "antd";
import { useAppContext } from "../../state";
import {
  base64Column,
  getSheetData,
  listUndoableOperations,
} from "../../tauri";
import { PAGE_SIZE } from "../../constants";

const { Title } = Typography;

// v1.1.2 加解密面板：所有加解密 / 编解码 / 哈希类操作的统一入口。
// 当前承载 Base64 编解码（就地变换，可撤销），后续将扩展 URL-safe Base64 /
// MD5 / SHA / AES 等命令。Base64 逻辑从 ColumnOpsPanel 迁入，行为不变：
//   base64Column → getSheetData + SET_SHEET_DATA 刷新当前页 +
//   listUndoableOperations + SET_UNDO_STACK 刷新撤销栈。
// 参考 MaskPanel/ExtractPanel 模式（Form + Select + Button + message +
// 操作后 getSheetData + dispatch SET_SHEET_DATA 刷新）。
export default function CryptoPanel() {
  const { state, dispatch } = useAppContext();
  const [base64Form] = Form.useForm();
  const [base64ing, setBase64ing] = useState(false);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];

  async function handleBase64() {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = base64Form.getFieldValue("column");
    const mode = base64Form.getFieldValue("mode");
    if (!column) {
      message.warning("请选择目标列");
      return;
    }
    if (!mode) {
      message.warning("请选择模式");
      return;
    }
    setBase64ing(true);
    try {
      const res = await base64Column(sheet.id, column, mode);
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
      const modeLabel = mode === "encode" ? "编码" : "解码";
      message.success(
        `Base64 ${modeLabel}完成：${res.affected} 行，跳过 ${res.skipped ?? 0}`
      );
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("base64_column failed:", e);
      message.error(`操作失败：${e}`);
    } finally {
      setBase64ing(false);
    }
  }

  return (
    <div style={{ padding: 4 }}>
      <Title level={5} style={{ marginTop: 0 }}>
        加解密
      </Title>
      <Form form={base64Form} layout="vertical" size="small">
        <Form.Item label="Base64 编解码 — 目标列" name="column">
          <Select
            placeholder="选择要编解码的列"
            options={headers.map((h) => ({ label: h, value: h }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        <Form.Item label="模式" name="mode" initialValue="encode">
          <Select
            options={[
              { label: "编码", value: "encode" },
              { label: "解码", value: "decode" },
            ]}
          />
        </Form.Item>
        <Form.Item>
          <Button
            type="primary"
            loading={base64ing}
            onClick={handleBase64}
            block
          >
            执行 Base64
          </Button>
        </Form.Item>
      </Form>
    </div>
  );
}
