import { Form, Input, Select, Switch, Tag, Typography } from "antd";

// phonePrefix（extract/validate 共用）：Tag + 允许前缀 + 正则输入框。
// draftAllowedPrefixes 空 = 不过滤前缀（11 位纯数字均放行）。
export default function PhonePrefixParams({
  draftPattern,
  setDraftPattern,
  draftAllowedPrefixes,
  setDraftAllowedPrefixes,
}) {
  const { Text } = Typography;
  return (
    <>
      <Form.Item label="校验类型">
        <Tag color="blue" style={{ margin: 0 }}>
          手机前缀
        </Tag>
      </Form.Item>
      <Form.Item label="允许前缀" extra="3 位前缀回车添加，留空=不限">
        <Select
          mode="tags"
          value={draftAllowedPrefixes}
          onChange={(v) => setDraftAllowedPrefixes(v)}
          placeholder="回车添加前缀"
          tokenSeparators={[",", " ", "\n"]}
          open={false}
          style={{ width: "100%" }}
        />
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
