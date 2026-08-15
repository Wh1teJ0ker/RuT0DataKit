import { Form, Input, Switch, Tag, Typography } from "antd";

// idcard-extract：只读 Tag + "允许首位为 0" 开关 + 正则输入框。
// 仅对提取规则生效（idcard-validate 无 pattern，不显示）。
export default function IdcardParams({
  draftPattern,
  setDraftPattern,
  draftAllowLeadingZero,
  setDraftAllowLeadingZero,
}) {
  const { Text } = Typography;
  return (
    <>
      <Form.Item label="校验类型">
        <Tag color="blue" style={{ margin: 0 }}>
          身份证
        </Tag>
      </Form.Item>
      <Form.Item label="允许首位为 0">
        <Switch
          checked={draftAllowLeadingZero}
          onChange={(checked) => {
            setDraftAllowLeadingZero(checked);
            setDraftPattern(
              checked
                ? "\\b\\d{17}[\\dXx]\\b"
                : "\\b[1-9]\\d{16}[\\dXx]\\b"
            );
          }}
        />
        <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
          {draftAllowLeadingZero
            ? "首位可为 0（宽松召回）"
            : "首位必须非零（默认）"}
        </Text>
      </Form.Item>
      <Form.Item label="正则模式">
        <Input
          value={draftPattern ?? ""}
          onChange={(e) => setDraftPattern(e.target.value)}
          placeholder="提取正则"
        />
      </Form.Item>
    </>
  );
}
