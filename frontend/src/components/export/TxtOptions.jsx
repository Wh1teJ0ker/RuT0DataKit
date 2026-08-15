import { Row, Col, Typography, Select, Input, Divider } from "antd";

// 行尾下拉项（TXT）。
const LINE_ENDING_OPTIONS = [
  { value: "crlf", label: "CRLF (Windows)" },
  { value: "lf", label: "LF (Unix)" },
];

// TXT 导出模式下拉项。
const TXT_MODE_OPTIONS = [
  { value: "merge", label: "合并行（每行数据 → 一行）" },
  { value: "percol", label: "逐列（每列各一行）" },
];

// TXT 专属选项：模式 + 模板 + 行尾。
export function TxtOptions({
  txtTemplate,
  setTxtTemplate,
  txtLineEnding,
  setTxtLineEnding,
  txtMode,
  setTxtMode,
}) {
  return (
    <>
      <Divider style={{ margin: 0 }} />
      <div>
        <Typography.Text>导出模式：</Typography.Text>
        <Select
          value={txtMode}
          onChange={(v) => {
            setTxtMode(v);
            if (v === "percol") {
              setTxtTemplate("{字段名}_{值}");
            } else {
              setTxtTemplate("{类型}_{数据值}");
            }
          }}
          style={{ width: "100%", marginTop: 4 }}
          options={TXT_MODE_OPTIONS}
        />
      </div>
      <div>
        <Typography.Text>TXT 模板：</Typography.Text>
        <Input.TextArea
          value={txtTemplate}
          onChange={(e) => setTxtTemplate(e.target.value)}
          placeholder={txtMode === "merge" ? "{类型}_{数据值}" : "{字段名}_{值}"}
          autoSize={{ minRows: 2, maxRows: 4 }}
          style={{ fontFamily: "monospace", marginTop: 4 }}
        />
        {txtMode === "merge" ? (
          <>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              合并行模式：用 `{"{列名}"}` 直接引用具体列；每行数据只渲染一行，多列值合并到同一行。
            </Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12, display: "block" }}>
              示例：`{"{类型}_{数据值}"}` → ip_163.211.48.156
            </Typography.Text>
          </>
        ) : (
          <>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              逐列模式：用 `{"{字段名}"}` 引用列名、`{"{值}"}` 引用单元格值；每行数据的每个选中列各渲染一行。
            </Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12, display: "block" }}>
              示例：`{"{字段名}_{值}"}` → 类型_ip（每列一行）
            </Typography.Text>
          </>
        )}
      </div>
      <Row gutter={[8, 8]}>
        <Col xs={{ span: 24 }} sm={{ span: 12 }}>
          <Typography.Text>行尾：</Typography.Text>
          <Select
            value={txtLineEnding}
            onChange={setTxtLineEnding}
            style={{ width: "100%", marginTop: 4 }}
            options={LINE_ENDING_OPTIONS}
          />
        </Col>
      </Row>
    </>
  );
}
