import { useState } from "react";
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
import { importFile, getSheetData } from "../../tauri";

// 右组能力按钮配置：id 与 state.activeCapability 取值一致。
const CAPABILITIES = [
  { id: "mask", label: "脱敏", icon: <SafetyCertificateOutlined /> },
  { id: "validate", label: "校验", icon: <CheckCircleOutlined /> },
  { id: "extract", label: "提取", icon: <FileSearchOutlined /> },
  { id: "rules", label: "规则管理", icon: <ControlOutlined /> },
];

// 左组数据操作：v1.0.0 仅「导入」可用，其余点击提示 v1.1+ 释放。
// 「导入」真实导入流（文件选择 → 写 DB → 渲染 Table）由 T5 接入。
const LEFT_OPS_DISABLED = [
  { key: "export", label: "导出", tip: "导出能力 v1.1+ 释放" },
  { key: "format", label: "格式", tip: "格式能力 v1.1+ 释放" },
  { key: "undo", label: "撤销", tip: "撤销能力 v1.1+ 释放" },
  { key: "column", label: "列操作", tip: "列操作能力 v1.1+ 释放" },
  { key: "run", label: "运行", tip: "运行能力 v1.1+ 释放" },
];

export default function TopToolbar({ activeCapability, setActiveCapability, onImport }) {
  const [importing, setImporting] = useState(false);

  async function handleImport() {
    if (importing) return;
    setImporting(true);
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: "Data", extensions: ["csv", "xlsx"] },
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
    </div>
  );
}

function iconFor(key) {
  switch (key) {
    case "export":
      return <ExportOutlined />;
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
