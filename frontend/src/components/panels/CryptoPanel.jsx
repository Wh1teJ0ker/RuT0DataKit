import { useState } from "react";
import { Alert, Button, Form, Radio, Select, Typography, message } from "antd";
import { useAppContext } from "../../state";
import { base64Column, hashColumn } from "../../tauri";
import { useSheetOps } from "../../hooks/useSheetOps";
import ColumnSelect from "../shared/ColumnSelect";

const { Title } = Typography;

// v1.1.2 加解密面板：所有加解密 / 编解码 / 哈希类操作的统一入口。
// 当前承载：
//   - Base64 编 / 解码（就地变换，可撤销）
//   - MD5 / SHA1 / SHA256 哈希（就地变换，可撤销；哈希不可逆，但通过
//     before_snapshot 回写可撤销恢复原文）
// 后续将扩展 URL-safe Base64 / AES 等命令。
// 操作模式：目标列 + 操作类型（op）二级选择，统一「执行」按钮按 op 分发。
// v1.2.0 T94：op-then-refresh 样板收敛到 useSheetOps.refreshActiveSheet，
// 列 Select 收敛到 ColumnSelect。
export default function CryptoPanel() {
  const { state, dispatch } = useAppContext();
  const { refreshActiveSheet } = useSheetOps(dispatch, null);
  const [form] = Form.useForm();
  const [running, setRunning] = useState(false);
  // v1.1.5 T83: 哈希输出大小写（Lower=小写 / Upper=大写）
  const [hashCase, setHashCase] = useState("lower");

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
        // md5 / sha1 / sha256 — 传 case 给后端
        res = await hashColumn(sheet.id, column, op, hashCase);
        const algoLabel = { md5: "Md5", sha1: "Sha1", sha256: "Sha256" }[op];
        label = `${algoLabel} 哈希（${hashCase === "upper" ? "大写" : "小写"}）`;
      }
      // 刷新当前页数据 + 撤销栈
      await refreshActiveSheet(sheet);
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
          <ColumnSelect headers={headers} placeholder="选择列" />
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
              description="不可逆，可通过撤销恢复原文。"
            />
          </Form.Item>
        )}
        {isHashOp && (
          <Form.Item label="输出大小写">
            <Radio.Group
              value={hashCase}
              onChange={(e) => setHashCase(e.target.value)}
            >
              <Radio value="lower">小写</Radio>
              <Radio value="upper">大写</Radio>
            </Radio.Group>
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
