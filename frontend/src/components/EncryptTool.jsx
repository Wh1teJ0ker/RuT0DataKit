import React, { useMemo, useState } from "react";
import {
  Card,
  Select,
  Radio,
  Input,
  Button,
  Table,
  Checkbox,
  Typography,
  Space,
  Empty,
  Alert,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  LockOutlined,
  UnlockOutlined,
} from "@ant-design/icons";
import {
  encryptText,
  decryptText,
  encryptColumns,
  decryptColumns,
} from "../tauri.js";

const { TextArea } = Input;
const { Text, Paragraph } = Typography;

// 加密 / 解密子界面（v0.6.0 T15-2，Tools Tab 第三项）。
//
// 三段结构：
//   ① 顶部配置：算法 Select（aes/base64/hex）+ 模式 Radio（加密/解密）
//      + Key Input.Password（仅 AES 显示；base64/hex 忽略 key）。
//   ② 单值试运行：TextArea 输入 + 运行按钮 → encryptText/decryptText
//      → Paragraph copyable 展示结果。AES 模式下 Key 为空时前端 message.warning。
//   ③ 批量列操作：若 state.records 存在，显示列勾选 Checkbox.Group
//      + 运行按钮 → encryptColumns/decryptColumns → antd Table 预览前 50 行。
//
// state/dispatch 从 props 透传；切 view 不重置，切 Tab 不清子状态（与 sql/regex 一致）。
//
// 算法别名对齐后端 encrypt.rs parse_algo：aes / aes-cbc / base64 / hex，
// 前端 Select value 用 "aes" / "base64" / "hex"。

const ENCRYPT_PREVIEW_ROW_LIMIT = 50;

export default function EncryptTool({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [selectedColumns, setSelectedColumns] = useState([]);

  const algo = state.encryptAlgo || "aes";
  const mode = state.encryptMode || "encrypt";
  const key = state.encryptKey || "";
  const input = state.encryptInput || "";
  const result = state.encryptResult;
  const columnsResult = state.encryptColumnsResult;
  const loading = state.encryptLoading || false;

  const isAes = algo === "aes";

  // records 来自 PreprocessView 导入产物（与 MaskView/ValidateView 同源）。
  const records = state.records || null;
  const hasRecords =
    records && Array.isArray(records.headers) && records.headers.length > 0;
  const headers = hasRecords ? records.headers : [];
  const rows = hasRecords ? records.rows : [];

  // AES 校验：Key 为空时阻止调用并提示。
  const ensureKey = () => {
    if (isAes && !key) {
      message.warning("AES 模式需要填写 Key");
      return false;
    }
    return true;
  };

  // ─────────────────────────────────────────────────────────────────────
  // ② 单值试运行
  // ─────────────────────────────────────────────────────────────────────
  const onRunSingle = async () => {
    if (!input) {
      message.warning("请输入要处理的文本");
      return;
    }
    if (!ensureKey()) return;
    dispatch({ type: "SET_ENCRYPT_LOADING", encryptLoading: true });
    try {
      const r =
        mode === "encrypt"
          ? await encryptText(algo, input, key)
          : await decryptText(algo, input, key);
      dispatch({ type: "SET_ENCRYPT_RESULT", encryptResult: r });
      if (r && r.ok === false && r.error) {
        message.error(`处理失败: ${r.error}`);
      }
    } catch (e) {
      message.error(`处理失败: ${e}`);
      dispatch({ type: "SET_ENCRYPT_RESULT", encryptResult: null });
    } finally {
      dispatch({ type: "SET_ENCRYPT_LOADING", encryptLoading: false });
    }
  };

  // ─────────────────────────────────────────────────────────────────────
  // ③ 批量列操作
  // ─────────────────────────────────────────────────────────────────────
  const onRunColumns = async () => {
    if (!hasRecords) {
      message.warning("请先到数据预处理导入文件");
      return;
    }
    if (selectedColumns.length === 0) {
      message.warning("请至少勾选一列");
      return;
    }
    if (!ensureKey()) return;
    dispatch({ type: "SET_ENCRYPT_LOADING", encryptLoading: true });
    try {
      const r =
        mode === "encrypt"
          ? await encryptColumns(headers, rows, algo, key, selectedColumns)
          : await decryptColumns(headers, rows, algo, key, selectedColumns);
      dispatch({
        type: "SET_ENCRYPT_COLUMNS_RESULT",
        encryptColumnsResult: r,
      });
      if (r && r.summary && r.summary.errors > 0) {
        message.warning(
          `处理完成，其中 ${r.summary.errors} 个 cell 失败（已原样保留）`
        );
      }
    } catch (e) {
      message.error(`批量处理失败: ${e}`);
      dispatch({
        type: "SET_ENCRYPT_COLUMNS_RESULT",
        encryptColumnsResult: null,
      });
    } finally {
      dispatch({ type: "SET_ENCRYPT_LOADING", encryptLoading: false });
    }
  };

  // 批量结果 Table 数据源（前 50 行）。
  const columnPreviewData = useMemo(() => {
    if (!columnsResult || !Array.isArray(columnsResult.processed_rows)) return [];
    return columnsResult.processed_rows
      .slice(0, ENCRYPT_PREVIEW_ROW_LIMIT)
      .map((row, idx) => {
        const o = { key: idx };
        (columnsResult.headers || []).forEach((h, c) => {
          o[h] = row[c] != null ? row[c] : "";
        });
        return o;
      });
  }, [columnsResult]);

  const columnPreviewColumns = useMemo(() => {
    if (!columnsResult) return [];
    return (columnsResult.headers || []).map((h) => ({
      title: h,
      dataIndex: h,
      key: h,
      ellipsis: true,
    }));
  }, [columnsResult]);

  const checkboxOptions = useMemo(
    () => headers.map((h) => ({ label: h, value: h })),
    [headers]
  );

  return (
    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
      {/* ① 顶部配置 */}
      <Card title="加密 / 解密配置" styles={{ body: { padding: 12 } }}>
        <Space size="middle" wrap align="center">
          <Space size="small" align="center">
            <Text type="secondary" style={{ fontSize: 12 }}>
              算法
            </Text>
            <Select
              value={algo}
              onChange={(v) =>
                dispatch({ type: "SET_ENCRYPT_ALGO", encryptAlgo: v })
              }
              style={{ width: 140 }}
              options={[
                { value: "aes", label: "AES-CBC" },
                { value: "base64", label: "Base64" },
                { value: "hex", label: "Hex" },
              ]}
            />
          </Space>
          <Space size="small" align="center">
            <Text type="secondary" style={{ fontSize: 12 }}>
              模式
            </Text>
            <Radio.Group
              value={mode}
              onChange={(e) =>
                dispatch({
                  type: "SET_ENCRYPT_MODE",
                  encryptMode: e.target.value,
                })
              }
              optionType="button"
              buttonStyle="solid"
            >
              <Radio.Button value="encrypt">加密</Radio.Button>
              <Radio.Button value="decrypt">解密</Radio.Button>
            </Radio.Group>
          </Space>
          {isAes && (
            <Space size="small" align="center">
              <Text type="secondary" style={{ fontSize: 12 }}>
                Key
              </Text>
              <Input.Password
                value={key}
                onChange={(e) =>
                  dispatch({
                    type: "SET_ENCRYPT_KEY",
                    encryptKey: e.target.value,
                  })
                }
                placeholder="AES 密钥（任意长度，SHA-256 派生 AES-256）"
                style={{ width: 320 }}
              />
            </Space>
          )}
        </Space>
        {isAes && (
          <div style={{ marginTop: 8 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              AES 使用随机 IV（内嵌于密文前缀），同一明文每次加密结果不同；解密时 Key 必须与加密时一致。
            </Text>
          </div>
        )}
      </Card>

      {/* ② 单值试运行 */}
      <Card
        title={`单值试运行（${mode === "encrypt" ? "加密" : "解密"}）`}
        styles={{ body: { padding: 12 } }}
      >
        <Space direction="vertical" size="small" style={{ width: "100%" }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            输入一段文本，先验证算法 / Key 是否符合预期，再进行批量列操作。
          </Text>
          <TextArea
            value={input}
            onChange={(e) =>
              dispatch({
                type: "SET_ENCRYPT_INPUT",
                encryptInput: e.target.value,
              })
            }
            placeholder={
              mode === "encrypt" ? "输入明文" : "输入密文 / 编码串"
            }
            autoSize={{ minRows: 2, maxRows: 6 }}
          />
          <Button
            type="primary"
            icon={
              mode === "encrypt" ? <LockOutlined /> : <UnlockOutlined />
            }
            loading={loading}
            onClick={onRunSingle}
          >
            运行
          </Button>
          {result ? (
            <Card
              size="small"
              title="结果"
              styles={{ body: { padding: 12 } }}
            >
              {result.ok ? (
                <Paragraph
                  copyable
                  style={{ fontFamily: "monospace", marginBottom: 0, wordBreak: "break-all" }}
                >
                  {result.result}
                </Paragraph>
              ) : (
                <Alert
                  type="error"
                  showIcon
                  message={result.error || "处理失败"}
                />
              )}
            </Card>
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description="运行后此处显示结果"
            />
          )}
        </Space>
      </Card>

      {/* ③ 批量列操作 */}
      <Card
        title={`批量列操作（${mode === "encrypt" ? "加密" : "解密"}）`}
        styles={{ body: { padding: 12 } }}
      >
        {hasRecords ? (
          <Space direction="vertical" size="small" style={{ width: "100%" }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              勾选要处理的列，未勾选列原样保留。下方表格预览处理结果前 {ENCRYPT_PREVIEW_ROW_LIMIT} 行。
            </Text>
            <Checkbox.Group
              options={checkboxOptions}
              value={selectedColumns}
              onChange={setSelectedColumns}
              style={{ width: "100%" }}
            />
            <Space size="middle" wrap>
              <Button
                type="primary"
                icon={<PlayCircleOutlined />}
                loading={loading}
                onClick={onRunColumns}
              >
                运行
              </Button>
              <Button
                onClick={() => setSelectedColumns(headers)}
                disabled={loading}
              >
                全选
              </Button>
              <Button
                onClick={() => setSelectedColumns([])}
                disabled={loading}
              >
                清空
              </Button>
            </Space>
            {columnsResult ? (
              <>
                {columnsResult.summary && (
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    共 {columnsResult.summary.total_rows} 行 / 处理{" "}
                    {columnsResult.summary.processed_cells} cell
                    {columnsResult.summary.errors > 0
                      ? ` / 失败 ${columnsResult.summary.errors}`
                      : ""}
                    {columnsResult.summary.skipped_fields_count > 0
                      ? ` / 跳过 ${columnsResult.summary.skipped_fields_count} 个不存在的列`
                      : ""}
                  </Text>
                )}
                <Table
                  size="small"
                  pagination={false}
                  scroll={{ y: 360, x: "max-content" }}
                  sticky
                  columns={columnPreviewColumns}
                  dataSource={columnPreviewData}
                  locale={{ emptyText: "无数据" }}
                />
              </>
            ) : (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description="勾选列后点击「运行」查看预览"
              />
            )}
          </Space>
        ) : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description="请先到数据预处理导入文件"
          />
        )}
      </Card>
    </Space>
  );
}
