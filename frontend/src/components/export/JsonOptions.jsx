import { Row, Col, Typography, Select, Divider } from "antd";

// JSON 缩进下拉项。
const INDENT_OPTIONS = [
  { value: 2, label: "2 空格" },
  { value: 4, label: "4 空格" },
  { value: 0, label: "紧凑（单行）" },
];

// JSON 结构下拉项。
const JSON_FORMAT_OPTIONS = [
  { value: "array", label: "数组 [{},{}]" },
  { value: "ndjson", label: "NDJSON（每行一对象）" },
];

// JSON 专属选项：缩进 + 结构。
export function JsonOptions({
  jsonIndent,
  setJsonIndent,
  jsonFormat,
  setJsonFormat,
}) {
  return (
    <>
      <Divider style={{ margin: 0 }} />
      <Row gutter={[8, 8]}>
        <Col xs={{ span: 24 }} sm={{ span: 12 }}>
          <Typography.Text>缩进：</Typography.Text>
          <Select
            value={jsonIndent}
            onChange={setJsonIndent}
            style={{ width: "100%", marginTop: 4 }}
            options={INDENT_OPTIONS}
          />
        </Col>
        <Col xs={{ span: 24 }} sm={{ span: 12 }}>
          <Typography.Text>结构：</Typography.Text>
          <Select
            value={jsonFormat}
            onChange={setJsonFormat}
            style={{ width: "100%", marginTop: 4 }}
            options={JSON_FORMAT_OPTIONS}
          />
        </Col>
      </Row>
    </>
  );
}
