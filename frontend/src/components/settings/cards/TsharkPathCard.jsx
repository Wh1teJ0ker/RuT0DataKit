import { useEffect, useCallback } from "react";
import {
  Button,
  Card,
  Descriptions,
  Input,
  Space,
  Typography,
  message,
} from "antd";
import { open } from "@tauri-apps/plugin-dialog";
import {
  detectTshark,
  loadTsharkPath,
  saveTsharkPath,
} from "../../../tauri";
import { useAppState, ACTION } from "../../../state";

const { Text } = Typography;

// 设置页 - tshark 路径卡片（v1.0.0 全格式扩展：激活）。
//
// 能力：
// - 启动时自动加载已保存路径（loadTsharkPath）。
// - 「自动检测」按钮：调 core::pcap::detect_tshark 多平台探测。
// - 「使用检测路径」：把检测到的 path 写入 state + settings.json。
// - 文件选择器：手动选 tshark 可执行文件。
// - 「清除」：清空覆盖，回退到 PATH 中的 tshark。
//
// 全本地探测，不外发数据。
export default function TsharkPathCard() {
  const { state, dispatch } = useAppState();
  const { tsharkPath, tsharkDetected, tsharkLoading } = state;

  // 启动时加载已保存路径。
  useEffect(() => {
    (async () => {
      try {
        const saved = await loadTsharkPath();
        if (saved) {
          dispatch({ type: ACTION.SET_TSHARK_PATH, payload: saved });
        }
      } catch {
        // 静默忽略（开发态无 IPC）
      }
    })();
  }, [dispatch]);

  const handleDetect = useCallback(async () => {
    dispatch({ type: ACTION.SET_TSHARK_LOADING, payload: true });
    try {
      const info = await detectTshark();
      if (info) {
        dispatch({ type: ACTION.SET_TSHARK_DETECTED, payload: info });
        message.success(`检测到 tshark ${info.version}`);
      } else {
        dispatch({ type: ACTION.SET_TSHARK_DETECTED, payload: null });
        message.warning("未检测到 tshark，请手动指定路径或安装 Wireshark");
      }
    } catch (e) {
      message.error(`检测失败：${String(e)}`);
    } finally {
      dispatch({ type: ACTION.SET_TSHARK_LOADING, payload: false });
    }
  }, [dispatch]);

  const handleUseDetected = useCallback(async () => {
    if (!tsharkDetected?.path) {
      message.warning("尚未检测到 tshark");
      return;
    }
    try {
      await saveTsharkPath(tsharkDetected.path);
      dispatch({ type: ACTION.SET_TSHARK_PATH, payload: tsharkDetected.path });
      message.success("已应用检测到的 tshark 路径");
    } catch (e) {
      message.error(`保存失败：${String(e)}`);
    }
  }, [tsharkDetected, dispatch]);

  const handlePickFile = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        // macOS Wireshark.app 内 tshark 无扩展名，filters 留空允许选任意文件。
      });
      if (!selected) return;
      const path = typeof selected === "string" ? selected : selected.path;
      if (!path) return;
      await saveTsharkPath(path);
      dispatch({ type: ACTION.SET_TSHARK_PATH, payload: path });
      message.success("已保存 tshark 路径");
    } catch (e) {
      message.error(`选择失败：${String(e)}`);
    }
  }, [dispatch]);

  const handleClear = useCallback(async () => {
    try {
      await saveTsharkPath(null);
      dispatch({ type: ACTION.SET_TSHARK_PATH, payload: null });
      message.info("已清除 tshark 路径覆盖，回退到 PATH 中的 tshark");
    } catch (e) {
      message.error(`清除失败：${String(e)}`);
    }
  }, [dispatch]);

  return (
    <Card title="tshark 路径" size="small">
      <Space direction="vertical" style={{ width: "100%" }} size="middle">
        <Space wrap>
          <Button onClick={handleDetect} loading={tsharkLoading}>
            自动检测
          </Button>
          <Button
            onClick={handleUseDetected}
            disabled={!tsharkDetected?.path}
          >
            使用检测路径
          </Button>
          <Button onClick={handlePickFile}>选择文件…</Button>
          <Button onClick={handleClear} disabled={!tsharkPath}>
            清除
          </Button>
        </Space>

        {tsharkDetected && (
          <Descriptions size="small" column={1} bordered>
            <Descriptions.Item label="检测路径">
              {tsharkDetected.path}
            </Descriptions.Item>
            <Descriptions.Item label="版本">
              {tsharkDetected.version}
            </Descriptions.Item>
          </Descriptions>
        )}

        <div>
          <Text type="secondary">当前覆盖路径：</Text>
          <Input
            value={tsharkPath || ""}
            placeholder="(未设置，回退到 PATH 中的 tshark)"
            readOnly
            style={{ marginTop: 4 }}
          />
        </div>

        <Text type="secondary">
          pcap 导入依赖 tshark。未配置时按 PATH 查找；缺失则 pcap 导入禁用。
          全本地处理，不上传任何 pcap/规则/样本。
        </Text>
      </Space>
    </Card>
  );
}
