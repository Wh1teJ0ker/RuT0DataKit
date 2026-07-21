import React, { useMemo } from "react";
import {
  Card,
  Table,
  Button,
  Typography,
  Space,
  Descriptions,
  Empty,
  App as AntApp,
} from "antd";
import {
  UploadOutlined,
  SearchOutlined,
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  CodeOutlined,
} from "@ant-design/icons";
import { tauriInvoke, preprocessFile } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";

const { Text } = Typography;

// 数据预处理主视图（v0.4.0 入口）：
//   ① 顶部导入按钮（调 select_file + preprocess_file）
//   ② Descriptions 概览（源类型 / 行数 / 列数）
//   ③ antd Table 预览（headers + 前 200 行）
//   ④ 底部跳转按钮组（搜索 / 数据脱敏 / 数据校验 / SQL 解析）
// 导入产物写入 state.records（SET_RECORDS），切 view 不重置；跳转按钮仅 dispatch SET_VIEW。
export default function PreprocessView({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const { records, loading } = state;

  const showError = (msg) => message.error(String(msg));

  const handleImport = async () => {
    dispatch({ type: "SET_LOADING", loading: true });
    try {
      const chosen = await tauriInvoke("select_file");
      if (!chosen) {
        dispatch({ type: "SET_LOADING", loading: false });
        return;
      }
      try {
        const res = await preprocessFile(chosen);
        const headers = res.headers || [];
        const rows = res.rows || [];
        const rowCount =
          res.row_count != null ? res.row_count : rows.length;
        const sourceType = res.source_type || null;
        dispatch({
          type: "SET_RECORDS",
          records: { headers, rows, rowCount, sourceType },
        });
        dispatch({ type: "SET_HINT", actionHint: "" });
      } catch (e) {
        showError(`预处理失败: ${e}`);
      }
    } catch (e) {
      showError(String(e));
    } finally {
      dispatch({ type: "SET_LOADING", loading: false });
    }
  };

  // 段 ③ 预览 Table：取 records.headers / records.rows 前 PREVIEW_ROW_LIMIT 行。
  const previewData = useMemo(() => {
    if (!records) return [];
    return records.rows.slice(0, PREVIEW_ROW_LIMIT).map((row, idx) => {
      const o = { key: idx };
      records.headers.forEach((h, c) => {
        o[h] = row[c] != null ? row[c] : "";
      });
      return o;
    });
  }, [records]);

  const previewColumns = useMemo(() => {
    if (!records) return [];
    return records.headers.map((h) => ({
      title: h,
      dataIndex: h,
      key: h,
      ellipsis: true,
    }));
  }, [records]);

  // 段 ④ 跳转按钮组：点击 dispatch SET_VIEW 切到对应 view，records 不丢。
  // 搜索 / SQL 解析（Tools）当前 view 未实现，由 App.jsx 渲染占位 Card。
  const jumps = [
    {
      key: "search",
      label: "搜索",
      icon: <SearchOutlined />,
      view: "search",
    },
    {
      key: "mask",
      label: "数据脱敏",
      icon: <SafetyCertificateOutlined />,
      view: "mask",
    },
    {
      key: "validate",
      label: "数据校验",
      icon: <CheckCircleOutlined />,
      view: "validate",
    },
    {
      key: "sql",
      label: "SQL 解析",
      icon: <CodeOutlined />,
      view: "tools",
    },
  ];

  return (
    <Card title="数据预处理" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {/* 段 ① 导入 */}
        <Card title="导入" styles={{ body: { padding: 12 } }}>
          <Space size="middle" wrap>
            <Button
              type="primary"
              icon={<UploadOutlined />}
              loading={loading}
              onClick={handleImport}
            >
              导入文件
            </Button>
            <Text type="secondary" style={{ fontSize: 12 }}>
              支持 csv / xlsx / sql / json / pcap / log，自动识别源类型
            </Text>
          </Space>
        </Card>

        {/* 段 ② 概览 */}
        {records ? (
          <Card title="概览" styles={{ body: { padding: 12 } }}>
            <Descriptions size="small" column={3} bordered>
              <Descriptions.Item label="源类型">
                {records.sourceType || "-"}
              </Descriptions.Item>
              <Descriptions.Item label="行数">
                {records.rowCount}
              </Descriptions.Item>
              <Descriptions.Item label="列数">
                {records.headers.length}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        ) : null}

        {/* 段 ③ 预览 */}
        <Card
          title="预览"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              {records
                ? `共 ${records.rowCount} 行 / 显示前 ${PREVIEW_ROW_LIMIT} 行`
                : "导入文件后此处显示预览"}
            </Text>
          }
        >
          {records ? (
            <Table
              size="small"
              pagination={false}
              scroll={{ y: 360, x: "max-content" }}
              sticky
              columns={previewColumns}
              dataSource={previewData}
            />
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description="点击上方「导入文件」加载预览"
            />
          )}
        </Card>

        {/* 段 ④ 跳转按钮组 */}
        <Card title="跳转" styles={{ body: { padding: 12 } }}>
          <Space size="middle" wrap>
            {jumps.map((j) => (
              <Button
                key={j.key}
                icon={j.icon}
                disabled={!records}
                onClick={() => dispatch({ type: "SET_VIEW", activeView: j.view })}
              >
                {j.label}
              </Button>
            ))}
          </Space>
          <div style={{ marginTop: 8 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {records
                ? "点击跳转后预处理产物保留，对应视图可直接消费"
                : "请先导入文件后启用跳转"}
            </Text>
          </div>
        </Card>
      </Space>
    </Card>
  );
}
