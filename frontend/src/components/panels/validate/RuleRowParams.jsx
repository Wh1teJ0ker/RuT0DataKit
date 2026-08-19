import { Checkbox, Input, InputNumber, Space, Form } from "antd";
import ColumnSelect from "../../shared/ColumnSelect";
import PhonePrefixSelect from "../../shared/PhonePrefixSelect";
import { BIRTH_FORMATS } from "../rule/constants";

// v1.1.5 T88：birth-validate 行级生日格式勾选。
// 全不选 = 接受所有格式（默认，向后兼容）；勾选后仅校验勾选的格式。
// 4 种 shouldUpdate 分支的参数组件。每个子组件监听该行 ruleId 变化，
// 仅匹配时渲染。通过 React Context 隐式获取 Form 实例（Form.Item 必须在 <Form> 内）。
export function RuleRowParams({ name, headers }) {
  return (
    <>
      {/* 跨字段配置：仅 idcard-validate 时展开。 */}
      <Form.Item
        shouldUpdate={(prev, cur) =>
          prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
        }
        noStyle
      >
        {({ getFieldValue }) =>
          getFieldValue(["rules", name, "ruleId"]) === "idcard-validate" ? (
            <Space direction="vertical" style={{ width: "100%", marginTop: 4 }}>
              <Space>
                <Form.Item name={[name, "checkSex"]} valuePropName="checked" noStyle>
                  <Checkbox>对比性别一致性</Checkbox>
                </Form.Item>
                <Form.Item name={[name, "sexColumn"]} noStyle>
                  <ColumnSelect headers={headers} placeholder="性别列" allowClear />
                </Form.Item>
              </Space>
              <Space>
                <Form.Item name={[name, "checkBirth"]} valuePropName="checked" noStyle>
                  <Checkbox>对比出生日期一致性</Checkbox>
                </Form.Item>
                <Form.Item name={[name, "birthColumn"]} noStyle>
                  <ColumnSelect headers={headers} placeholder="出生日期列" allowClear />
                </Form.Item>
              </Space>
            </Space>
          ) : null
        }
      </Form.Item>

      {/* generic-validate 行级参数配置。 */}
      <Form.Item
        shouldUpdate={(prev, cur) =>
          prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
        }
        noStyle
      >
        {({ getFieldValue }) =>
          getFieldValue(["rules", name, "ruleId"]) === "generic-validate" ? (
            <Space direction="vertical" style={{ width: "100%", marginTop: 4 }}>
              <Form.Item name={[name, "charClasses"]} label="允许的字符类" style={{ marginBottom: 0 }}>
                <Checkbox.Group
                  options={[
                    { label: "纯数字", value: "digits" },
                    { label: "纯字母", value: "letters" },
                  ]}
                />
              </Form.Item>
              <Form.Item name={[name, "specialChars"]} label="允许的特殊符号" style={{ marginBottom: 0 }}>
                <Input placeholder="留空=不允许；如 _-.@" allowClear />
              </Form.Item>
              <Space>
                <Form.Item name={[name, "minLen"]} label="最小长度" noStyle>
                  <InputNumber placeholder="不限" min={0} style={{ flex: 1, minWidth: 80 }} />
                </Form.Item>
                <Form.Item name={[name, "maxLen"]} label="最大长度" noStyle>
                  <InputNumber placeholder="不限" min={0} style={{ flex: 1, minWidth: 80 }} />
                </Form.Item>
              </Space>
            </Space>
          ) : null
        }
      </Form.Item>

      {/* phone-validate 行级前缀白名单配置。 */}
      <Form.Item
        shouldUpdate={(prev, cur) =>
          prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
        }
        noStyle
      >
        {({ getFieldValue }) =>
          getFieldValue(["rules", name, "ruleId"]) === "phone-validate" ? (
            <Form.Item
              name={[name, "phonePrefixes"]}
              label="手机号前缀白名单"
              style={{ marginTop: 4, marginBottom: 0 }}
              extra="三位数字前缀，留空=不限"
            >
              <PhonePrefixSelect />
            </Form.Item>
          ) : null
        }
      </Form.Item>

      {/* birth-validate 行级生日格式勾选。 */}
      <Form.Item
        shouldUpdate={(prev, cur) =>
          prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
        }
        noStyle
      >
        {({ getFieldValue }) =>
          getFieldValue(["rules", name, "ruleId"]) === "birth-validate" ? (
            <Form.Item
              name={[name, "birthFormats"]}
              label="生日格式"
              style={{ marginTop: 4, marginBottom: 0 }}
              extra="全不选=接受所有格式"
            >
              <Checkbox.Group options={BIRTH_FORMATS} />
            </Form.Item>
          ) : null
        }
      </Form.Item>

      {/* address-validate 行级号/室范围配置。 */}
      <Form.Item
        shouldUpdate={(prev, cur) =>
          prev.rules?.[name]?.ruleId !== cur.rules?.[name]?.ruleId
        }
        noStyle
      >
        {({ getFieldValue }) =>
          getFieldValue(["rules", name, "ruleId"]) === "address-validate" ? (
            <Space direction="vertical" style={{ width: "100%", marginTop: 4 }}>
              <Space>
                <Form.Item name={[name, "minHao"]} label="「号」最小" noStyle>
                  <InputNumber placeholder="不限" min={0} style={{ flex: 1, minWidth: 80 }} />
                </Form.Item>
                <Form.Item name={[name, "maxHao"]} label="最大" noStyle>
                  <InputNumber placeholder="不限" min={0} style={{ flex: 1, minWidth: 80 }} />
                </Form.Item>
              </Space>
              <Space>
                <Form.Item name={[name, "minShi"]} label="「室」最小" noStyle>
                  <InputNumber placeholder="不限" min={0} style={{ flex: 1, minWidth: 80 }} />
                </Form.Item>
                <Form.Item name={[name, "maxShi"]} label="最大" noStyle>
                  <InputNumber placeholder="不限" min={0} style={{ flex: 1, minWidth: 80 }} />
                </Form.Item>
              </Space>
            </Space>
          ) : null
        }
      </Form.Item>
    </>
  );
}
