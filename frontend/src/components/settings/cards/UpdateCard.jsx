import { useState } from "react";
import { Button, Card, Space, Typography, message } from "antd";
import { SyncOutlined, DownloadOutlined } from "@ant-design/icons";
import { checkUpdate, installUpdate } from "../../../tauri";

const { Text, Paragraph } = Typography;

// 设置页 - 更新检查卡片。
// T14：raw invoke('check_update') / invoke('install_update') 已收口到 tauri.js 的
// checkUpdate() / installUpdate()，前端唯一 IPC 出口保持集中（见 docs/02 §4）。
// check_update 永不抛错给前端（无网络 / 无新版本 / 接口异常统一降级 available=false），
// 但前端仍兜底 try/catch，避免 invoke 本身异常导致崩溃。
export default function UpdateCard() {
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState(false);
  // status 形如 { available, version, notes }（camelCase，见 src-tauri/src/commands/update.rs UpdateStatus）
  const [status, setStatus] = useState(null);
  const [error, setError] = useState(null);

  async function handleCheck() {
    setLoading(true);
    setError(null);
    try {
      const res = await checkUpdate();
      setStatus(res);
    } catch (e) {
      // 前端兜底：invoke 层异常也不崩溃，降级为不可用。
      setError(String(e));
      setStatus({ available: false, version: null, notes: null });
    } finally {
      setLoading(false);
    }
  }

  async function handleInstall() {
    setInstalling(true);
    try {
      await installUpdate();
      message.success("更新已安装，重启后生效");
    } catch (e) {
      message.error(`安装失败：${String(e)}`);
    } finally {
      setInstalling(false);
    }
  }

  return (
    <Card title="检查更新" size="small">
      <Space direction="vertical" style={{ width: "100%" }}>
        <Button
          icon={<SyncOutlined spin={loading} />}
          loading={loading}
          onClick={handleCheck}
        >
          检查更新
        </Button>

        {error ? (
          <Text type="secondary">（检查失败，已降级为不可用）</Text>
        ) : status ? (
          status.available ? (
            <Space direction="vertical" style={{ width: "100%" }}>
              <Text>
                发现新版本：<Text strong>{status.version}</Text>
              </Text>
              {status.notes ? (
                <Paragraph
                  type="secondary"
                  ellipsis={{ rows: 4, expandable: true, symbol: "展开" }}
                  style={{ marginBottom: 0 }}
                >
                  {status.notes}
                </Paragraph>
              ) : null}
              <Button
                type="primary"
                icon={<DownloadOutlined />}
                loading={installing}
                onClick={handleInstall}
              >
                安装更新
              </Button>
            </Space>
          ) : (
            <Text type="secondary">当前已是最新版本</Text>
          )
        ) : (
          <Text type="secondary">点击按钮检查是否有新版本</Text>
        )}
      </Space>
    </Card>
  );
}
