import { Form, Input, Tag, Typography } from "antd";
import { VALIDATE_LABELS } from "../constants";

// 无 params 的 pattern-only 分支（如 name-validate）：hint 文案 + 正则输入框。
// 带只读 validator Tag 的分支（luhn/ipv4/ipv6/username/sex/birth/address）。
export default function PatternParams({
  selected,
  draftPattern,
  setDraftPattern,
  hint = "",
}) {
  const { Text } = Typography;
  // 其他 validator（luhn/ipv4/ipv6/username/sex/birth/address）：
  // 只读 Tag + hint 文案，不显示空正则输入框。
  if (selected.params?.validator) {
    return (
      <>
        <Form.Item label="校验类型">
          <Tag color="blue" style={{ margin: 0 }}>
            {VALIDATE_LABELS[selected.params.validator] ||
              selected.params.validator}
          </Tag>
        </Form.Item>
        <Text type="secondary">{hint}</Text>
      </>
    );
  }
  // 无 params 的 validate 规则（如 name-validate）：hint 文案 + 正则输入框。
  return (
    <>
      {selected.kind === "validate" && hint && (
        <Text type="secondary" style={{ display: "block", marginBottom: 8 }}>
          {hint}
        </Text>
      )}
      <Form.Item label="正则模式">
        <Input
          value={draftPattern ?? ""}
          onChange={(e) => setDraftPattern(e.target.value)}
          placeholder={
            selected.kind === "validate"
              ? "如 ^[\u4e00-\u9fa5]{2,4}$"
              : "如 [\u4e00-\u9fa5]{2,4}"
          }
        />
      </Form.Item>
    </>
  );
}
