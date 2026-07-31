import { Button, Card, Space, Typography, message } from "antd";
import { FolderOpenOutlined } from "@ant-design/icons";
import { DEV_STATUS } from "../../../constants";

const { Text, Paragraph } = Typography;

// 设置页 - DB 路径展示卡片。
// v1.0.0 不许动 src-tauri/，无法新增 get_db_path 命令（见 TASK-T8-HANDOFF §risks）。
// 故前端硬编码展示 macOS 预期路径 + 说明文案；后续版本可补 get_db_path 命令。
// 「在 Finder 中显示」：Tauri v2 shell 插件需注册，v1.0.0 未注册，按钮提示「开发中」，
// 不为此加 src-tauri 改动。
const DB_PATH = "~/Library/Application Support/com.rut0.datakit/ruT0datakit.db";

export default function DbPathCard() {
  return (
    <Card title="数据库路径" size="small">
      <Space direction="vertical" style={{ width: "100%" }}>
        <Paragraph
          code
          copyable
          style={{ marginBottom: 0, wordBreak: "break-all" }}
        >
          {DB_PATH}
        </Paragraph>
        <Text type="secondary">
          这是 v1.0.0 在 macOS 上的预期数据库位置；后续版本将提供动态获取命令。
        </Text>
        <Button
          icon={<FolderOpenOutlined />}
          onClick={() =>
            message.info(`「在 Finder 中显示」${DEV_STATUS}`)
          }
        >
          在 Finder 中显示
        </Button>
      </Space>
    </Card>
  );
}
