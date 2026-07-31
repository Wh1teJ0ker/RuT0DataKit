import { useState, useCallback } from "react";
import { Button, Divider, Space, message } from "antd";
import {
  ImportOutlined,
  ExportOutlined,
  FormatPainterOutlined,
  UndoOutlined,
  ColumnHeightOutlined,
  PlayCircleOutlined,
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  FileSearchOutlined,
  ControlOutlined,
} from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { importFile } from "../../tauri";
import { DEV_STATUS } from "../../constants";
import { useAppContext } from "../../state";
import ExportModal from "../ExportModal";

// 右组能力按钮配置：id 与 state.activeCapability 取值一致。
const CAPABILITIES = [
  { id: "mask", label: "脱敏", icon: <SafetyCertificateOutlined /> },
  { id: "validate", label: "校验", icon: <CheckCircleOutlined /> },
  { id: "extract", label: "提取", icon: <FileSearchOutlined /> },
  { id: "rules", label: "规则管理", icon: <ControlOutlined /> },
];

// 左组数据操作：v1.0.0「导入 + 导出」可用，其余点击提示「开发中」。
// 「导入」真实导入流（文件选择 → 写 DB → 渲染 Table）由 T5 接入。
// 「导出」v1.0.0 客户端 CSV 导出（Blob 下载），不新增 IPC。
const LEFT_OPS_DISABLED = [
  { key: "format", label: "格式", tip: `格式能力 ${DEV_STATUS}` },
  { key: "undo", label: "撤销", tip: `撤销能力 ${DEV_STATUS}` },
  { key: "column", label: "列操作", tip: `列操作能力 ${DEV_STATUS}` },
  { key: "run", label: "运行", tip: `运行能力 ${DEV_STATUS}` },
];

// T13：state/dispatch/setter 经 useAppContext 取，仅保留 onImport 跨组件回调。
export default function TopToolbar({ onImport }) {
  const { state, setActiveCapability } = useAppContext();
  const { activeCapability } = state;
  const activeSheet = state.sheets.find(
    (s) => s.id === state.activeSheetId
  );

  const [importing, setImporting] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);

  async function handleImport() {
    if (importing) return;
    setImporting(true);
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Data",
            extensions: [
              "csv",
              "xlsx",
              "json",
              "jsonl",
              "sql",
              "txt",
              "pcap",
              "pcapng",
            ],
          },
        ],
      });
      if (!selected) return; // 用户取消
      const path = typeof selected === "string" ? selected : selected.path;
      if (!path) return;
      const result = await importFile(path);
      // result: { sessionId, sheetId, rowCount, headers }
      const importPayload = {
        sessionId: result.sessionId,
        sheetId: result.sheetId,
        rowCount: result.rowCount,
        headers: result.headers,
        name: path.split(/[\\/]/).pop(),
      };
      onImport?.(importPayload);
      message.success(`导入完成：${result.rowCount} 行`);
    } catch (e) {
      message.error(`导入失败：${String(e)}`);
    } finally {
      setImporting(false);
    }
  }

  const handleExport = useCallback(() => {
    if (!activeSheet) {
      message.warning("请先导入数据再导出");
      return;
    }
    setExportOpen(true);
  }, [activeSheet]);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        height: "100%",
        padding: "0 16px",
        gap: 12,
      }}
    >
      <Space size={4}>
        <Button
          icon={<ImportOutlined />}
          loading={importing}
          onClick={handleImport}
        >
          导入
        </Button>
        <Button
          icon={<ExportOutlined />}
          onClick={handleExport}
          disabled={!activeSheet}
        >
          导出
        </Button>
        {LEFT_OPS_DISABLED.map((op) => (
          <Button
            key={op.key}
            icon={iconFor(op.key)}
            onClick={() => message.info(op.tip)}
          >
            {op.label}
          </Button>
        ))}
      </Space>

      <Divider type="vertical" style={{ height: 24, margin: "0 4px" }} />

      <Space size={4}>
        {CAPABILITIES.map((cap) => (
          <Button
            key={cap.id}
            icon={cap.icon}
            type={activeCapability === cap.id ? "primary" : "default"}
            onClick={() => setActiveCapability(cap.id)}
          >
            {cap.label}
          </Button>
        ))}
      </Space>

      <ExportModal
        open={exportOpen}
        sheet={activeSheet}
        onClose={() => setExportOpen(false)}
      />
    </div>
  );
}

function iconFor(key) {
  switch (key) {
    case "format":
      return <FormatPainterOutlined />;
    case "undo":
      return <UndoOutlined />;
    case "column":
      return <ColumnHeightOutlined />;
    case "run":
      return <PlayCircleOutlined />;
    default:
      return null;
  }
}
