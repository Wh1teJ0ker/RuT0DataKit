import React from "react";
import { Card, Form, Select, Input, Button, Typography, Space } from "antd";
import { PlusOutlined, CheckOutlined } from "@ant-design/icons";
import { MASKER_DEFS, getMaskerDef } from "../maskerDefs.js";

const { Text } = Typography;
const { TextArea } = Input;

const PARAM_LABELS = {
  pattern: "pattern",
  replacement: "replacement",
  keep_prefix: "keep_prefix",
  keep_suffix: "keep_suffix",
  mask_char: "mask_char",
  mask_min_len: "mask_min_len",
  with: "with",
};

// 规则表单：支持新增 / 编辑两种模式。
// props.editingRule 传入时进入编辑模式（回填 field/masker/params/description，
// 提交调 UPDATE_MASK_RULE + SET_EDITING_MASK_RULE(null)；否则调 ADD_MASK_RULE）。
// 提交后 reset 表单。
export default function RuleForm({ state, dispatch, editingRule }) {
  const [form] = Form.useForm();
  const headers = state.headers;
  const isEdit = !!editingRule;

  const masker = Form.useWatch("masker", form) || MASKER_DEFS[0].name;
  const def = getMaskerDef(masker);

  const fieldOptions = headers.map((h) => ({ label: h, value: h }));
  const maskerOptions = MASKER_DEFS.map((m) => ({ label: m.name, value: m.name }));

  // editingRule 变化时回填表单。
  React.useEffect(() => {
    if (editingRule) {
      const values = {
        field: editingRule.field,
        masker: editingRule.masker,
        description: editingRule.description || "",
      };
      for (const p of def.params) {
        values[p] =
          editingRule.params && editingRule.params[p] != null
            ? editingRule.params[p]
            : "";
      }
      form.setFieldsValue(values);
    } else {
      form.resetFields();
      form.setFieldsValue({ masker: MASKER_DEFS[0].name });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editingRule]);

  const handleFinish = (values) => {
    if (!values.field) {
      return;
    }
    const params = {};
    const curDef = getMaskerDef(values.masker);
    for (const p of curDef.params) {
      params[p] = values[p] != null ? values[p] : "";
    }
    const rule = {
      field: values.field,
      masker: values.masker,
      params,
      description: values.description ? values.description : undefined,
    };
    if (isEdit) {
      dispatch({ type: "UPDATE_MASK_RULE", index: editingRule.__index, rule });
      dispatch({ type: "SET_EDITING_MASK_RULE", rule: null });
    } else {
      dispatch({ type: "ADD_MASK_RULE", rule });
    }
    // reset
    form.resetFields();
    form.setFieldsValue({ masker: MASKER_DEFS[0].name });
  };

  return (
    <Card title={isEdit ? "编辑规则" : "添加规则"} bodyStyle={{ padding: 12 }}>
      <Form
        form={form}
        layout="inline"
        initialValues={{ masker: MASKER_DEFS[0].name }}
        onFinish={handleFinish}
      >
        <Form.Item
          label="字段 field"
          name="field"
          rules={[{ required: true, message: "请选择字段" }]}
        >
          <Select
            placeholder={headers.length ? "选择字段" : "请先导入文件"}
            options={fieldOptions}
            style={{ minWidth: 200 }}
            notFoundContent="请先导入文件"
            showSearch
          />
        </Form.Item>
        <Form.Item
          label="masker"
          name="masker"
          rules={[{ required: true, message: "请选择 masker" }]}
        >
          <Select options={maskerOptions} style={{ minWidth: 180 }} showSearch />
        </Form.Item>
        {def.params.length === 0 ? (
          <Text type="secondary" style={{ fontSize: 12 }}>
            该 masker 无参数
          </Text>
        ) : (
          <Space size="small" wrap>
            {def.params.map((p) => (
              <Form.Item key={p} label={PARAM_LABELS[p] || p} name={p}>
                <Input placeholder={p} style={{ width: 160 }} />
              </Form.Item>
            ))}
          </Space>
        )}
        <Form.Item label="描述" name="description">
          <TextArea
            placeholder="可选：规则说明"
            autoSize={{ minRows: 1, maxRows: 3 }}
            style={{ width: 240 }}
          />
        </Form.Item>
        <Form.Item>
          <Button
            type="primary"
            icon={isEdit ? <CheckOutlined /> : <PlusOutlined />}
            htmlType="submit"
          >
            {isEdit ? "保存修改" : "添加规则"}
          </Button>
          {isEdit ? (
            <Button
              style={{ marginLeft: 8 }}
              onClick={() => {
                dispatch({ type: "SET_EDITING_MASK_RULE", rule: null });
                form.resetFields();
                form.setFieldsValue({ masker: MASKER_DEFS[0].name });
              }}
            >
              取消
            </Button>
          ) : null}
        </Form.Item>
      </Form>
    </Card>
  );
}
