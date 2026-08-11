import { useState } from "react";
import { Alert, Button, Form, Select, Typography, message } from "antd";
import { useAppContext } from "../../state";
import {
  base64Column,
  getSheetData,
  hashColumn,
  listUndoableOperations,
} from "../../tauri";
import { PAGE_SIZE } from "../../constants";

const { Title } = Typography;

// v1.1.2 加解密面板：所有加解密 / 编解码 / 哈希类操作的统一入口。
// 当前承载：
//   - Base64 编 / 解码（就地变换，可撤销）
//   - MD5 / SHA1 / SHA256 哈希（就地变换，可撤销；哈希不可逆，但通过
//     before_snapshot 回写可撤销恢复原文）
// 后续将扩展 URL-safe Base64 / AES 等命令。
// 操作模式：目标列 + 操作类型（op）二级选择，统一「执行」按钮按 op 分发。
// 执行流程：
//   base64Column / hashColumn → getSheetData + SET_SHEET_DATA 刷新当前页 +
//   listUndoableOperations + SET_UNDO_STACK 刷新撤销栈 + message.success。
// 参考 MaskPanel/ExtractPanel 模式（Form + Select + Button + message +
// 操作后 getSheetData + dispatch SET_SHEET_DATA 刷新）。
export default function CryptoPanel() {
  const { state, dispatch } = useAppContext();
  const [form] = Form.useForm();
  const [running, setRunning] = useState(false);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];

  async function handleExecute() {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = form.getFieldValue("column");
    const op = form.getFieldValue("op");
    if (!column) {
      message.warning("请选择目标列");
      return;
    }
    if (!op) {
      message.warning("请选择操作类型");
      return;
    }
    setRunning(true);
    try {
      let res;
      let label;
      if (op.startsWith("base64_")) {
        const mode = op === "base64_encode" ? "encode" : "decode";
        res = await base64Column(sheet.id, column, mode);
        label = `Base64 ${mode === "encode" ? "编码" : "解码"}`;
      } else {
        // md5 / sha1 / sha256
        res = await hashColumn(sheet.id, column, op);
        const algoLabel = { md5: "MD5", sha1: "SHA1", sha256: "SHA256" }[op];
        label = `${algoLabel} 哈希`;
      }
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
      message.success(
        `${label}完成：${res.affected} 行，跳过 ${res.skipped ?? 0}`
      );
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("crypto operation failed:", e);
      message.error(`操作失败：${e}`);
    } finally {
      setRunning(false);
    }
  }

  // 选中的 op 是否为哈希（用于动态展示 hint 文案）
  const currentOp = Form.useWatch("op", form);
  const isHashOp =
    currentOp === "md5" || currentOp === "sha1" || currentOp === "sha256";

  return (
    <div style={{ padding: 4 }}>
      <Title level={5} style={{ marginTop: 0 }}>
        加解密
      </Title>
      <Form form={form} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
          <Select
            placeholder="选择要操作的列"
            options={headers.map((h) => ({ label: h, value: h }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        <Form.Item label="操作类型" name="op" initialValue="base64_encode">
          <Select
            options={[
              { label: "Base64 编码", value: "base64_encode" },
              { label: "Base64 解码", value: "base64_decode" },
              { label: "MD5 哈希", value: "md5" },
              { label: "SHA1 哈希", value: "sha1" },
              { label: "SHA256 哈希", value: "sha256" },
            ]}
          />
        </Form.Item>
        {isHashOp && (
          <Form.Item>
            <Alert
              type="warning"
              showIcon
              message="哈希不可逆"
              description="哈希操作无法解码还原，但可通过撤销恢复原文。"
            />
          </Form.Item>
        )}
        <Form.Item>
          <Button
            type="primary"
            loading={running}
            onClick={handleExecute}
            block
          >
            执行
          </Button>
        </Form.Item>
      </Form>
    </div>
  );
}
