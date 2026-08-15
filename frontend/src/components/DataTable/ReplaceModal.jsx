import { Modal, Form, Input, Switch } from "antd";

// 全局替换 Modal：查找 + 替换 + 正则开关。
// onOk 由 DataTable.handleReplace 注入（replaceAll → refreshActiveSheet → CLEAR_SEARCH）。
export function ReplaceModal({ open, form, replacing, onOk, onCancel }) {
  return (
    <Modal
      title="全局替换"
      open={open}
      onCancel={onCancel}
      onOk={onOk}
      confirmLoading={replacing}
      okText="替换"
      cancelText="取消"
      destroyOnClose
    >
      <Form form={form} layout="vertical" size="small">
        <Form.Item label="查找" name="from" rules={[{ required: true }]}>
          <Input allowClear autoComplete="off" autoCapitalize="off" spellCheck={false} />
        </Form.Item>
        <Form.Item label="替换为" name="to">
          <Input allowClear autoComplete="off" autoCapitalize="off" spellCheck={false} />
        </Form.Item>
        <Form.Item label="正则" name="useRegex" valuePropName="checked">
          <Switch />
        </Form.Item>
      </Form>
    </Modal>
  );
}
