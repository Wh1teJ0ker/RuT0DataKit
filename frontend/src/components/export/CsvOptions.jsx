import { Row, Col, Typography, Select, Switch, Input, Divider } from "antd";

// 导出格式下拉项。CSV/JSON/TXT 可用；XLSX 已移除。
const SEPARATOR_OPTIONS = [
  { value: "comma", label: '逗号 ","' },
  { value: "semicolon", label: '分号 ";"' },
  { value: "tab", label: "Tab" },
  { value: "pipe", label: '竖线 "|"' },
  { value: "custom", label: "自定义" },
];

// CSV 专属选项：分隔符 + 表头开关。
export function CsvOptions({
  csvSeparator,
  setCsvSeparator,
  csvCustomSeparator,
  setCsvCustomSeparator,
  csvWithHeader,
  setCsvWithHeader,
}) {
  return (
    <>
      <Divider style={{ margin: 0 }} />
      <Row gutter={[8, 8]}>
        <Col xs={{ span: 24 }} sm={{ span: 12 }}>
          <Typography.Text>列分隔符：</Typography.Text>
          <Select
            value={csvSeparator}
            onChange={setCsvSeparator}
            style={{ width: "100%", marginTop: 4 }}
            options={SEPARATOR_OPTIONS}
          />
        </Col>
        <Col xs={{ span: 24 }} sm={{ span: 12 }}>
          <Typography.Text>含表头行：</Typography.Text>
          <div style={{ marginTop: 4 }}>
            <Switch checked={csvWithHeader} onChange={setCsvWithHeader} />
          </div>
        </Col>
      </Row>
      {csvSeparator === "custom" && (
        <Input
          value={csvCustomSeparator}
          onChange={(e) => setCsvCustomSeparator(e.target.value)}
          placeholder="自定义分隔符，如 | 或 #"
          style={{ marginTop: 4 }}
        />
      )}
    </>
  );
}
