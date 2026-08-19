import { Form, InputNumber, Tag, Typography } from "antd";

// v1.2.2：address-validate 范围编辑器（RulesPanel）。
// 与 ValidatePanel 的 RuleRowParams address 行级范围一致：
// 全留空 = 不限范围（向后兼容）；填入后校验「号」/「室」前数字范围。
export default function AddressParams({
  draftAddressParams,
  setDraftAddressParams,
  hint = "",
}) {
  const { Text } = Typography;
  const update = (key, val) =>
    setDraftAddressParams({ ...draftAddressParams, [key]: val });

  return (
    <>
      <Form.Item label="校验类型">
        <Tag color="blue" style={{ margin: 0 }}>
          地址
        </Tag>
      </Form.Item>
      <Form.Item label="「号」数字范围" extra="留空 = 不限">
        <InputNumber
          placeholder="最小"
          min={0}
          value={draftAddressParams.minHao}
          onChange={(v) => update("minHao", v)}
          style={{ width: 100 }}
        />
        <span style={{ margin: "0 8px" }}>~</span>
        <InputNumber
          placeholder="最大"
          min={0}
          value={draftAddressParams.maxHao}
          onChange={(v) => update("maxHao", v)}
          style={{ width: 100 }}
        />
      </Form.Item>
      <Form.Item label="「室」数字范围" extra="留空 = 不限">
        <InputNumber
          placeholder="最小"
          min={0}
          value={draftAddressParams.minShi}
          onChange={(v) => update("minShi", v)}
          style={{ width: 100 }}
        />
        <span style={{ margin: "0 8px" }}>~</span>
        <InputNumber
          placeholder="最大"
          min={0}
          value={draftAddressParams.maxShi}
          onChange={(v) => update("maxShi", v)}
          style={{ width: 100 }}
        />
      </Form.Item>
      <Text type="secondary">{hint}</Text>
    </>
  );
}
