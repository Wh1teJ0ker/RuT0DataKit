import { Card, Radio, Button, Input, Table, Tag, Space, Spin, App as AntApp } from "antd";
import {
  saveDialog,
  tauriInvoke,
  extractText,
  extractFile,
  exportExtract,
} from "../tauri.js";

const { TextArea } = Input;

// v0.4.3 T9-5 数据提取视图：4 Card（输入 / 操作 / 结果 / 导出）。
// 输入：Radio 切「文件导入」（选 .txt 路径）|「文本粘贴」（TextArea）。
// 操作：开始提取按钮——按 extractMode 调 extractFile / extractText，
//   返回 { findings: [{type, value}], counts: {phone, bankcard, ip} }。
// 结果：顶部计数 Tag（phone/bankcard/ip）+ antd Table（type/value 两列）。
// 导出：三按钮（txt/csv/json）调 saveDialog + exportExtract。
// loading 期间禁用按钮防重入；错误走 antd message。
export default function ExtractView({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const { extractMode, extractInput, extractResult, extractLoading } = state;

  const handleSelectFile = async () => {
    const path = await tauriInvoke("select_file");
    if (path) dispatch({ type: "SET_EXTRACT_INPUT", extractInput: path });
  };

  const handleExtract = async () => {
    if (!extractInput) {
      message.warning("请先选择文件或输入文本");
      return;
    }
    dispatch({ type: "SET_EXTRACT_LOADING", extractLoading: true });
    try {
      const result =
        extractMode === "file"
          ? await extractFile(extractInput)
          : await extractText(extractInput);
      dispatch({ type: "SET_EXTRACT_RESULT", extractResult: result });
      message.success(`提取完成：${result.findings.length} 条`);
    } catch (e) {
      message.error(`提取失败：${e}`);
    } finally {
      dispatch({ type: "SET_EXTRACT_LOADING", extractLoading: false });
    }
  };

  const handleExport = async (format) => {
    if (!extractResult?.findings?.length) {
      message.warning("无可导出的结果");
      return;
    }
    const outPath = await saveDialog(`extract.${format}`, format);
    if (!outPath) return;
    try {
      await exportExtract(extractResult.findings, format, outPath);
      message.success(`已导出：${outPath}`);
    } catch (e) {
      message.error(`导出失败：${e}`);
    }
  };

  const columns = [
    { title: "类型", dataIndex: "type", key: "type", width: 120 },
    { title: "值", dataIndex: "value", key: "value" },
  ];

  return (
    <Spin spinning={extractLoading}>
      <Card title="输入">
        <Radio.Group
          value={extractMode}
          onChange={(e) =>
            dispatch({ type: "SET_EXTRACT_MODE", extractMode: e.target.value })
          }
          style={{ marginBottom: 16 }}
        >
          <Radio.Button value="file">文件导入</Radio.Button>
          <Radio.Button value="text">文本粘贴</Radio.Button>
        </Radio.Group>
        {extractMode === "file" ? (
          <div>
            <Button onClick={handleSelectFile}>选择 .txt 文件</Button>
            {extractInput && (
              <span style={{ marginLeft: 16, color: "#8c8c8c" }}>
                {extractInput}
              </span>
            )}
          </div>
        ) : (
          <TextArea
            rows={10}
            value={extractMode === "text" ? extractInput : ""}
            onChange={(e) =>
              dispatch({
                type: "SET_EXTRACT_INPUT",
                extractInput: e.target.value,
              })
            }
            placeholder="粘贴含手机号/银行卡号/IP 的文本..."
          />
        )}
      </Card>

      <Card title="操作" style={{ marginTop: 16 }}>
        <Button type="primary" onClick={handleExtract} loading={extractLoading}>
          开始提取
        </Button>
      </Card>

      {extractResult && (
        <Card title="结果" style={{ marginTop: 16 }}>
          <Space style={{ marginBottom: 16 }}>
            <Tag color="blue">phone: {extractResult.counts.phone}</Tag>
            <Tag color="green">bankcard: {extractResult.counts.bankcard}</Tag>
            <Tag color="orange">ip: {extractResult.counts.ip}</Tag>
          </Space>
          <Table
            dataSource={extractResult.findings}
            columns={columns}
            rowKey={(r, i) => i}
            size="small"
            pagination={{ pageSize: 50 }}
          />
        </Card>
      )}

      {extractResult?.findings?.length > 0 && (
        <Card title="导出" style={{ marginTop: 16 }}>
          <Space>
            <Button onClick={() => handleExport("txt")}>导出 TXT</Button>
            <Button onClick={() => handleExport("csv")}>导出 CSV</Button>
            <Button onClick={() => handleExport("json")}>导出 JSON</Button>
          </Space>
        </Card>
      )}
    </Spin>
  );
}
