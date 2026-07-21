import React, { useEffect } from "react";
import {
  Card,
  Typography,
  Space,
  Button,
  Descriptions,
  Tag,
  App as AntApp,
  Alert,
} from "antd";
import {
  SearchOutlined,
  CheckOutlined,
  DeleteOutlined,
  FolderOpenOutlined,
} from "@ant-design/icons";
import { tauriInvoke, detectTshark, loadTsharkPath, saveTsharkPath } from "../tauri.js";

const { Text, Paragraph } = Typography;

// v0.4.2 T7-3 设置主视图。
// 首期唯一功能：tshark 多平台自动检测 + 路径配置持久化。
// 挂载时自动调 loadTsharkPath（灌入已保存路径）+ detectTshark（探测当前可用性）。
// 用户可：自动检测 / 使用检测到的路径 / 清除自定义路径 / 选择文件手选 tshark。
// 配置保存到 app_config_dir/settings.json，pcap 解析立即生效（core 全局覆盖路径）。
//
// 全本地处理，不调用网络（满足 docs/00 §6「不外发数据」）。
export default function SettingsView({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const { tsharkPath, tsharkDetected, tsharkLoading } = state;

  // 挂载时自动加载已保存路径 + 探测 tshark。失败不阻塞，仅 warning。
  useEffect(() => {
    let cancelled = false;
    (async () => {
      dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: true });
      try {
        const saved = await loadTsharkPath();
        if (cancelled) return;
        if (saved) {
          dispatch({ type: "SET_TSHARK_PATH", tsharkPath: saved });
        }
      } catch (e) {
        console.warn("load_tshark_path failed:", e);
      }
      try {
        const detected = await detectTshark();
        if (cancelled) return;
        dispatch({ type: "SET_TSHARK_DETECTED", tsharkDetected: detected });
      } catch (e) {
        console.warn("detect_tshark failed:", e);
        if (!cancelled) {
          message.warning(`tshark 探测失败: ${e}`);
        }
      } finally {
        if (!cancelled) {
          dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: false });
        }
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 状态 Tag：用户覆盖（蓝）/ 已检测（绿）/ 未检测（红）。
  const statusTag = (() => {
    if (tsharkPath) {
      return <Tag color="blue">用户覆盖</Tag>;
    }
    if (tsharkDetected?.path) {
      return <Tag color="green">已检测</Tag>;
    }
    if (tsharkDetected) {
      return <Tag color="red">未检测</Tag>;
    }
    return <Tag>未探测</Tag>;
  })();

  // 当前生效路径：用户覆盖优先，否则检测到的，否则空。
  const effectivePath = tsharkPath || tsharkDetected?.path || "-";

  const onDetect = async () => {
    dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: true });
    try {
      const detected = await detectTshark();
      dispatch({ type: "SET_TSHARK_DETECTED", tsharkDetected: detected });
      if (detected?.path) {
        message.success(`检测到 tshark: ${detected.path}`);
      } else {
        message.warning("未检测到 tshark，请尝试手选或安装 Wireshark CLI");
      }
    } catch (e) {
      message.error(`探测失败: ${e}`);
    } finally {
      dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: false });
    }
  };

  const onUseDetected = async () => {
    const p = tsharkDetected?.path;
    if (!p) {
      message.warning("当前未检测到 tshark，无法应用");
      return;
    }
    dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: true });
    try {
      await saveTsharkPath(p);
      dispatch({ type: "SET_TSHARK_PATH", tsharkPath: p });
      message.success(`已保存 tshark 路径: ${p}`);
    } catch (e) {
      message.error(`保存失败: ${e}`);
    } finally {
      dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: false });
    }
  };

  const onClear = async () => {
    dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: true });
    try {
      await saveTsharkPath(null);
      dispatch({ type: "SET_TSHARK_PATH", tsharkPath: null });
      message.success("已清除自定义路径，将回退到 PATH 中的 tshark");
    } catch (e) {
      message.error(`清除失败: ${e}`);
    } finally {
      dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: false });
    }
  };

  const onSelectFile = async () => {
    dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: true });
    try {
      const chosen = await tauriInvoke("select_file");
      if (!chosen) {
        // 用户取消
        return;
      }
      await saveTsharkPath(chosen);
      dispatch({ type: "SET_TSHARK_PATH", tsharkPath: chosen });
      message.success(`已保存 tshark 路径: ${chosen}`);
    } catch (e) {
      message.error(`保存失败: ${e}`);
    } finally {
      dispatch({ type: "SET_TSHARK_LOADING", tsharkLoading: false });
    }
  };

  return (
    <Card title="设置" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Card
          title="tshark 路径配置"
          size="small"
          styles={{ body: { padding: 12 } }}
          extra={<Text type="secondary" style={{ fontSize: 12 }}>v0.4.2</Text>}
        >
          <Space direction="vertical" size="middle" style={{ width: "100%" }}>
            <Descriptions size="small" column={1} bordered>
              <Descriptions.Item label="状态">
                {statusTag}
              </Descriptions.Item>
              <Descriptions.Item label="当前生效路径">
                <Text code copyable={effectivePath !== "-"}>
                  {effectivePath}
                </Text>
              </Descriptions.Item>
              <Descriptions.Item label="版本">
                <Text type="secondary">
                  {tsharkDetected?.version || "-"}
                </Text>
              </Descriptions.Item>
            </Descriptions>

            <Space size="middle" wrap>
              <Button
                type="primary"
                icon={<SearchOutlined />}
                onClick={onDetect}
                loading={tsharkLoading}
              >
                自动检测
              </Button>
              <Button
                icon={<CheckOutlined />}
                onClick={onUseDetected}
                disabled={!tsharkDetected?.path || tsharkLoading}
              >
                使用检测到的路径
              </Button>
              <Button
                icon={<FolderOpenOutlined />}
                onClick={onSelectFile}
                disabled={tsharkLoading}
              >
                选择文件...
              </Button>
              <Button
                danger
                icon={<DeleteOutlined />}
                onClick={onClear}
                disabled={!tsharkPath || tsharkLoading}
              >
                清除自定义路径
              </Button>
            </Space>

            <Alert
              type="info"
              showIcon
              message="配置后 pcap 解析将使用此路径；清除则回退到系统 PATH 中的 tshark。"
              description={
                <Paragraph style={{ marginBottom: 0, fontSize: 12 }}>
                  各平台常见路径参考：<br />
                  • macOS: <Text code>/opt/homebrew/bin/tshark</Text> /{" "}
                  <Text code>/Applications/Wireshark.app/Contents/MacOS/tshark</Text><br />
                  • Linux: <Text code>/usr/bin/tshark</Text> /{" "}
                  <Text code>/usr/local/bin/tshark</Text><br />
                  • Windows: <Text code>C:\Program Files\Wireshark\tshark.exe</Text>
                </Paragraph>
              }
            />
          </Space>
        </Card>
      </Space>
    </Card>
  );
}
