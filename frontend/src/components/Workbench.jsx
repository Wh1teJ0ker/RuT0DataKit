import { Empty, Layout, Typography } from "antd";

const { Content } = Layout;

// 中央 Workbench 占位。currentView === 'settings' 时由 T8 设置页覆盖；
// 本任务仅在 settings 态渲染占位文本，避免阻塞 T8。
export default function Workbench({ currentView }) {
  return (
    <Content
      style={{
        flex: 1,
        minWidth: 0,
        padding: 24,
        background: "#fff",
        overflow: "auto",
      }}
    >
      {currentView === "settings" ? (
        <Typography.Text type="secondary">
          设置页 v1.0.0 T8 释放
        </Typography.Text>
      ) : (
        <Empty
          style={{ marginTop: 64 }}
          description="导入数据后在此展示工作台"
        />
      )}
    </Content>
  );
}
