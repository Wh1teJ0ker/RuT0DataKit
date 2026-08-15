import { Alert, Button, Form, Input, List, Space, Typography } from "antd";

const { Text } = Typography;

// 内联测试 Card（前端纯逻辑，不写 DB）。
// 纯展示组件：输入框 + 执行/清空按钮 + 结果渲染。
// 测试逻辑（runMaskTest/runValidateTest/runExtractTest）由父组件实现。
export default function RuleTest({
  testInput,
  setTestInput,
  testResult,
  testing,
  onTest,
  defaultMaskChar,
}) {
  return (
    <>
      <Form layout="vertical" size="small">
        <Form.Item label="测试输入">
          <Input.TextArea
            rows={3}
            value={testInput}
            onChange={(e) => setTestInput(e.target.value)}
            placeholder="输入测试文本"
          />
        </Form.Item>
      </Form>
      <Space style={{ marginTop: 4 }}>
        <Button loading={testing} onClick={onTest}>
          执行测试
        </Button>
        <Button
          onClick={() => {
            setTestInput("");
          }}
        >
          清空
        </Button>
      </Space>

      {testResult && (
        <div
          style={{
            marginTop: 12,
            maxHeight: "30vh",
            overflow: "auto",
          }}
        >
          {testResult.error ? (
            <Alert type="error" showIcon message={testResult.error} />
          ) : testResult.kind === "validate" ? (
            <Alert
              type={testResult.passed ? "success" : "warning"}
              showIcon
              message={testResult.message}
              description={
                testResult.note ? `附加信息：${testResult.note}` : undefined
              }
            />
          ) : testResult.kind === "extract" ? (
            testResult.hits.length ? (
              <List
                size="small"
                dataSource={testResult.hits}
                style={{ maxHeight: "20vh", overflow: "auto" }}
                renderItem={(h, i) => (
                  <List.Item key={i}>
                    <Text code>{h.value}</Text>
                    <Text type="secondary" style={{ fontSize: 11 }}>
                      {" "}
                      （{h.start}-{h.end}）
                    </Text>
                  </List.Item>
                )}
              />
            ) : (
              <Alert type="info" showIcon message="无命中" />
            )
          ) : testResult.kind === "mask" ? (
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                原文：
              </Text>
              <Text code>{testResult.input || "（空）"}</Text>
              <br />
              <Text type="secondary" style={{ fontSize: 12 }}>
                掩码字符：
              </Text>
              <Text code>
                {testResult.maskChar || defaultMaskChar}
              </Text>
              <br />
              <Text type="secondary" style={{ fontSize: 12 }}>
                脱敏后：
              </Text>
              <Text code>{testResult.output}</Text>
              {testResult.template && testResult.passthrough && (
                <>
                  <br />
                  <Alert
                    type="info"
                    showIcon
                    style={{ marginTop: 8 }}
                    message="空模板，原样返回"
                  />
                </>
              )}
            </div>
          ) : null}
        </div>
      )}
    </>
  );
}
