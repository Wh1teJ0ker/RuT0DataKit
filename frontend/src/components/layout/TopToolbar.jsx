import { useState, useCallback } from "react";
import { Button, Divider, Space, message } from "antd";
import {
  ImportOutlined,
  ExportOutlined,
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  FileSearchOutlined,
  ControlOutlined,
  ColumnHeightOutlined,
  KeyOutlined,
  SettingOutlined,
  SafetyOutlined,
} from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { importFile } from "../../tauri";
import { useAppContext } from "../../state";
import ExportModal from "../ExportModal";

// 右组能力按钮配置：id 与 state.activeCapability 取值一致。
const CAPABILITIES = [
  { id: "mask", label: "脱敏", icon: <SafetyCertificateOutlined /> },
  { id: "validate", label: "校验", icon: <CheckCircleOutlined /> },
  { id: "extract", label: "提取", icon: <FileSearchOutlined /> },
  { id: "rowValidate", label: "行级校验", icon: <SafetyOutlined /> },
  { id: "columnOps", label: "列操作", icon: <ColumnHeightOutlined /> },
  { id: "crypto", label: "加解密", icon: <KeyOutlined /> },
  { id: "rules", label: "规则管理", icon: <ControlOutlined /> },
];

export default function TopToolbar({ onImport }) {
  const { state, setActiveCapability, setView } = useAppContext();
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
              "log",
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

      <div style={{ flex: 1 }} />

      <Button
        type="text"
        icon={<SettingOutlined />}
        onClick={() => setView("settings")}
      />

      <ExportModal
        open={exportOpen}
        sheet={activeSheet}
        onClose={() => setExportOpen(false)}
      />
    </div>
  );
}
