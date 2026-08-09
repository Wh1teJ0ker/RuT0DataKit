import { useState, useCallback } from "react";
import { Button, Card, Form, Select, Space, Typography, message } from "antd";
import { useAppState, ACTION } from "../../../state";
import { loadPageSize, savePageSize, getSheetData } from "../../../tauri";
import { PAGE_SIZE } from "../../../constants";

const { Text } = Typography;

// 设置页 - 全局每页行数卡片（v1.1.2）。
//
// 能力：
// - 启动时自动加载已保存的每页行数（loadPageSize）。
// - Select 常用档位：20 / 50 / 100 / 200（默认 50）。
// - 「保存」按钮：savePageSize → dispatch SET_PAGE_SIZE（reducer 同步所有 Sheet
//   的 pageSize 并重置 page=1）→ 刷新当前激活 Sheet 首页数据。
//
// 持久化到 settings.json（与 tshark 路径同文件），重启后仍生效。
const PAGE_SIZE_OPTIONS = [20, 50, 100, 200];

export default function PageSizeCard() {
  const { state, dispatch } = useAppState();
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm();

  // 初始值：全局 state.pageSize（App.jsx 启动时已 loadPageSize 覆盖）。
  // Form 受控，用户选择后由 form 保存，点「保存」按钮才提交。
  const currentValue = state.pageSize || PAGE_SIZE;

  const handleSave = useCallback(async () => {
    const value = form.getFieldValue("pageSize") || currentValue;
    setSaving(true);
    try {
      await savePageSize(value);
      dispatch({ type: ACTION.SET_PAGE_SIZE, payload: value });
      // 刷新当前激活 Sheet 首页数据（reducer 已把 page 重置为 1）。
      const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
      if (sheet && sheet.sessionId != null) {
        try {
          const data = await getSheetData(sheet.id, 1, value);
          dispatch({
            type: ACTION.SET_SHEET_DATA,
            payload: { ...data, sheetId: sheet.id },
          });
        } catch (e) {
          // 数据刷新失败不阻断设置保存，仅提示。
          // eslint-disable-next-line no-console
          console.error("getSheetData after page_size change failed:", e);
        }
      }
      message.success(`每页行数已设置为 ${value}`);
    } catch (e) {
      message.error(`保存失败：${String(e)}`);
    } finally {
      setSaving(false);
    }
  }, [form, currentValue, dispatch, state.sheets, state.activeSheetId]);

  return (
    <Card title="每页行数" size="small">
      <Space direction="vertical" style={{ width: "100%" }} size="middle">
        <Form form={form} layout="vertical" size="small" initialValues={{ pageSize: currentValue }}>
          <Form.Item
            label="全局每页显示行数"
            name="pageSize"
            extra="影响所有表格的分页、翻页与搜索翻页。新建/导入的 Sheet 也将继承此值。"
          >
            <Select
              options={PAGE_SIZE_OPTIONS.map((n) => ({ label: `${n} 行`, value: n }))}
            />
          </Form.Item>
          <Form.Item>
            <Button type="primary" loading={saving} onClick={handleSave} block>
              保存
            </Button>
          </Form.Item>
        </Form>
        <Text type="secondary">
          默认 50 行；设置后立即生效并持久化到 settings.json，重启后保持。
        </Text>
      </Space>
    </Card>
  );
}
