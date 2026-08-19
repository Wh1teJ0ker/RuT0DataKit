import { Checkbox, Form, Tag, Typography } from "antd";
import { BIRTH_FORMATS } from "../constants";

// v1.2.2：birth-validate 格式编辑器（RulesPanel）。
// 与 ValidatePanel 的 RuleRowParams birth 行级勾选一致：
// 全不选 = 接受所有格式（向后兼容）；勾选后仅校验勾选的格式。
export default function BirthParams({ draftBirthFormats, setDraftBirthFormats, hint = "" }) {
  const { Text } = Typography;
  return (
    <>
      <Form.Item label="校验类型">
        <Tag color="blue" style={{ margin: 0 }}>
          出生日期
        </Tag>
      </Form.Item>
      <Form.Item label="接受的日期格式" extra="全不选 = 接受所有格式">
        <Checkbox.Group
          value={draftBirthFormats}
          onChange={(vals) => setDraftBirthFormats(vals)}
          options={BIRTH_FORMATS}
        />
      </Form.Item>
      <Text type="secondary">{hint}</Text>
    </>
  );
}
