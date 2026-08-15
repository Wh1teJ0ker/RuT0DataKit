import { Checkbox, Form, Input, InputNumber, Tag, Typography } from "antd";

// v1.1.4 续轮 T71：generic-validate 字符类 + 长度限制。
// T77：特殊符号从 Checkbox 全开/全关改为自定义白名单 Input。
export default function GenericParams({
  draftGenericParams,
  setDraftGenericParams,
  hint = "",
}) {
  const { Text } = Typography;
  return (
    <>
      <Form.Item label="校验类型">
        <Tag color="blue" style={{ margin: 0 }}>
          通用校验
        </Tag>
      </Form.Item>
      <Form.Item label="允许的字符类">
        <Checkbox.Group
          value={[
            draftGenericParams.allowDigits && "digits",
            draftGenericParams.allowLetters && "letters",
          ].filter(Boolean)}
          onChange={(vals) =>
            setDraftGenericParams((prev) => ({
              ...prev,
              allowDigits: vals.includes("digits"),
              allowLetters: vals.includes("letters"),
            }))
          }
          options={[
            { label: "纯数字 (0-9)", value: "digits" },
            { label: "纯字母 (a-zA-Z)", value: "letters" },
          ]}
        />
      </Form.Item>
      <Form.Item label="允许的特殊符号">
        <Input
          value={draftGenericParams.allowSpecialChars}
          onChange={(e) =>
            setDraftGenericParams((prev) => ({
              ...prev,
              allowSpecialChars: e.target.value,
            }))
          }
          placeholder="留空=不允许；如 _-.@"
          allowClear
        />
      </Form.Item>
      <Form.Item label="最小长度（空=不限）">
        <InputNumber
          value={draftGenericParams.minLen}
          onChange={(v) =>
            setDraftGenericParams((prev) => ({
              ...prev,
              minLen: v,
            }))
          }
          placeholder="不限"
          min={0}
          style={{ width: "100%" }}
        />
      </Form.Item>
      <Form.Item label="最大长度（空=不限）">
        <InputNumber
          value={draftGenericParams.maxLen}
          onChange={(v) =>
            setDraftGenericParams((prev) => ({
              ...prev,
              maxLen: v,
            }))
          }
          placeholder="不限"
          min={0}
          style={{ width: "100%" }}
        />
      </Form.Item>
      <Text type="secondary">{hint}</Text>
    </>
  );
}
