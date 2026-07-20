import React, { useMemo } from "react";
import {
  Card,
  Table,
  Button,
  Typography,
  Space,
  Empty,
  Tooltip,
  Tag,
  Descriptions,
  Spin,
  App as AntApp,
} from "antd";
import { UploadOutlined, PlayCircleOutlined } from "@ant-design/icons";
import { tauriInvoke, scanPcapFile } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";

const { Text } = Typography;

// 流量分析视图（v0.3.0）：四段垂直布局，与 LogView 同模式。
//   ① 顶部：导入 .pcap/.pcapng 按钮（select_file）+ 文件路径显示
//   ② 原始 HTTP 请求表（state.pcapEntries.slice(0,200)，分页 pageSize=50）
//   ③ 「运行扫描」按钮 + summary 卡片（total_requests/sensitive_hits/
//     decoded_fragments/top_src_ips）
//   ④ findings 表（分页 pageSize=50）
// state/dispatch 从 props 透传；pcapEntries/pcapReport/pcapLoading 在全局
// useReducer state，切 view 不丢数据。
// tshark 缺失时后端返回 CoreError::DependencyMissing("tshark")，此处统一
// message.error 提示用户安装 Wireshark CLI。
export default function PcapView({ state, dispatch }) {
  const { message } = AntApp.useApp();

  // 导入 .pcap：select_file 拿路径 → 直接调 scanPcapFile 拿 {entries, report}。
  const handleSelectFile = async () => {
    dispatch({ type: "SET_PCAP_LOADING", pcapLoading: true });
    try {
      const chosen = await tauriInvoke("select_file");
      if (!chosen) {
        dispatch({ type: "SET_PCAP_LOADING", pcapLoading: false });
        return;
      }
      // 用 detect_source_type 校验是否为 pcap 类型。
      try {
        const t = await tauriInvoke("detect_source_type", { path: chosen });
        if (t !== "pcap") {
          message.warning(`流量分析仅支持 .pcap/.pcapng，当前识别为 ${t}`);
          dispatch({ type: "SET_PCAP_LOADING", pcapLoading: false });
          return;
        }
      } catch (e) {
        message.error(`类型识别失败: ${e}`);
        dispatch({ type: "SET_PCAP_LOADING", pcapLoading: false });
        return;
      }
      try {
        const res = await scanPcapFile(chosen);
        dispatch({
          type: "SET_FILE",
          filePath: chosen,
          sourceType: "pcap",
          headers: [],
          rows: [],
          rowCount: (res.entries || []).length,
        });
        dispatch({ type: "SET_PCAP_ENTRIES", pcapEntries: res.entries || [] });
        dispatch({ type: "SET_PCAP_REPORT", pcapReport: res.report || null });
        dispatch({ type: "SET_HINT", actionHint: "" });
      } catch (e) {
        // tshark 缺失提示统一处理
        const msg = String(e);
        if (msg.includes("tshark") || msg.includes("DependencyMissing")) {
          message.error(
            "流量分析需要系统 tshark，请先安装 Wireshark CLI (brew install wireshark)"
          );
        } else {
          message.error(`流量扫描失败: ${e}`);
        }
      }
    } catch (e) {
      message.error(String(e));
    } finally {
      dispatch({ type: "SET_PCAP_LOADING", pcapLoading: false });
    }
  };

  // 段 ② 原始 HTTP 请求表：slice 前 200 条，分页 50。
  const rawData = useMemo(() => {
    return (state.pcapEntries || [])
      .slice(0, PREVIEW_ROW_LIMIT)
      .map((e) => ({ ...e, key: e.frame_no }));
  }, [state.pcapEntries]);

  const truncateCell = (text, max = 40) => {
    if (!text) return "";
    const s = String(text);
    if (s.length <= max) return s;
    return (
      <Tooltip title={s}>
        <span>{s.slice(0, max) + "..."}</span>
      </Tooltip>
    );
  };

  const rawColumns = [
    { title: "帧", dataIndex: "frame_no", width: 70, fixed: "left" },
    { title: "源 IP", dataIndex: "src_ip", width: 140, ellipsis: true },
    { title: "目的 IP", dataIndex: "dst_ip", width: 140, ellipsis: true },
    { title: "Method", dataIndex: "method", width: 80 },
    { title: "Host", dataIndex: "host", width: 160, ellipsis: true },
    {
      title: "URI",
      dataIndex: "uri",
      ellipsis: true,
      render: (v) => truncateCell(v, 60),
    },
    {
      title: "Body 预览",
      dataIndex: "body",
      ellipsis: true,
      render: (v) =>
        v ? (
          <Tooltip title={String(v).slice(0, 200)}>
            <span>{String(v).slice(0, 50) + "..."}</span>
          </Tooltip>
        ) : (
          "-"
        ),
    },
    {
      title: "User-Agent",
      dataIndex: "user_agent",
      ellipsis: true,
      render: (v) => truncateCell(v, 40),
    },
  ];

  // 段 ③ 重新运行扫描：filePath 已在 state，直接调 scanPcapFile。
  const onRunScan = async () => {
    if (!state.filePath) {
      message.warning("请先导入 .pcap/.pcapng 文件");
      return;
    }
    dispatch({ type: "SET_PCAP_LOADING", pcapLoading: true });
    dispatch({ type: "SET_HINT", actionHint: "正在扫描..." });
    try {
      const res = await scanPcapFile(state.filePath);
      dispatch({ type: "SET_PCAP_ENTRIES", pcapEntries: res.entries || [] });
      dispatch({ type: "SET_PCAP_REPORT", pcapReport: res.report || null });
      dispatch({ type: "SET_HINT", actionHint: "扫描完成" });
    } catch (e) {
      const msg = String(e);
      if (msg.includes("tshark") || msg.includes("DependencyMissing")) {
        message.error(
          "流量分析需要系统 tshark，请先安装 Wireshark CLI (brew install wireshark)"
        );
        dispatch({ type: "SET_HINT", actionHint: "缺少 tshark" });
      } else {
        message.error(`扫描失败: ${e}`);
        dispatch({ type: "SET_HINT", actionHint: `扫描失败: ${e}` });
      }
    } finally {
      dispatch({ type: "SET_PCAP_LOADING", pcapLoading: false });
    }
  };

  // summary 取自 report.summary（serde_yml::Mapping → JSON 对象）。
  const summary = state.pcapReport?.summary || null;
  const topSrcIps = summary?.top_src_ips || [];

  // 段 ④ findings 表：分页 50。
  const findingsData = useMemo(() => {
    return (state.pcapReport?.findings || []).map((f, idx) => ({
      ...f,
      key: idx,
    }));
  }, [state.pcapReport]);

  const TAG_COLOR_BY_TYPE = {
    idcard: "purple",
    phone: "blue",
    bankcard: "magenta",
    email: "cyan",
    mac: "geekblue",
    username: "gold",
    name: "green",
  };
  const tagColor = (t) => TAG_COLOR_BY_TYPE[t] || "default";

  const findingsColumns = [
    {
      title: "类型",
      dataIndex: "type",
      width: 110,
      render: (t) => <Tag color={tagColor(t)}>{t}</Tag>,
    },
    {
      title: "值",
      dataIndex: "value",
      ellipsis: true,
      render: (v) => truncateCell(v, 80),
    },
    {
      title: "位置（帧）",
      dataIndex: "location",
      width: 120,
      render: (v) => v ?? "-",
    },
    {
      title: "上下文",
      dataIndex: "context",
      ellipsis: true,
      render: (v) => (v ? truncateCell(v, 80) : "-"),
    },
  ];

  return (
    <Card title="流量分析" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {/* 段 ① 导入按钮 + 文件路径 */}
        <Card title="导入流量包" styles={{ body: { padding: 12 } }}>
          <Space size="middle" align="center" wrap>
            <Button
              type="primary"
              icon={<UploadOutlined />}
              loading={state.pcapLoading}
              onClick={handleSelectFile}
            >
              导入 .pcap/.pcapng
            </Button>
            <Text
              type={state.filePath ? undefined : "secondary"}
              style={{
                flex: 1,
                minWidth: 200,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {state.filePath || "未选择文件"}
            </Text>
          </Space>
        </Card>

        {/* 段 ② 原始 HTTP 请求表 */}
        <Card
          title="原始 HTTP 请求"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              共 {state.pcapEntries.length} 条 / 显示前{" "}
              {Math.min(state.pcapEntries.length, PREVIEW_ROW_LIMIT)} 条
            </Text>
          }
        >
          <Table
            size="small"
            pagination={{ pageSize: 50, size: "small" }}
            scroll={{ y: 280, x: "max-content" }}
            sticky
            locale={{ emptyText: "导入 .pcap/.pcapng 后此处显示原始 HTTP 请求" }}
            columns={rawColumns}
            dataSource={rawData}
          />
        </Card>

        {/* 段 ③ 运行扫描按钮 + summary 卡片 */}
        <Card title="扫描结果" styles={{ body: { padding: 12 } }}>
          <Space size="middle" wrap style={{ marginBottom: 12 }}>
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              loading={state.pcapLoading}
              disabled={state.pcapEntries.length === 0}
              onClick={onRunScan}
            >
              运行扫描
            </Button>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {state.actionHint}
            </Text>
          </Space>
          <Spin spinning={state.pcapLoading}>
            {state.pcapReport ? (
              <Descriptions
                size="small"
                bordered
                column={4}
                items={[
                  {
                    key: "total_requests",
                    label: "HTTP 请求数",
                    children: summary?.total_requests ?? "-",
                  },
                  {
                    key: "sensitive_hits",
                    label: "敏感命中",
                    children: summary?.sensitive_hits ?? 0,
                  },
                  {
                    key: "decoded_fragments",
                    label: "解码片段",
                    children: summary?.decoded_fragments ?? 0,
                  },
                  {
                    key: "top_src_ips",
                    label: "源 IP TOP5",
                    span: 4,
                    children:
                      topSrcIps.length > 0 ? (
                        <Space size="small" wrap>
                          {topSrcIps.map((it, idx) => (
                            <Tag key={idx} color="volcano">
                              {it.ip} ({it.hits})
                            </Tag>
                          ))}
                        </Space>
                      ) : (
                        <Text type="secondary">无</Text>
                      ),
                  },
                ]}
              />
            ) : (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description="点击「运行扫描」生成报告"
              />
            )}
          </Spin>
        </Card>

        {/* 段 ④ findings 表 */}
        <Card
          title="Findings"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              共 {findingsData.length} 条
            </Text>
          }
        >
          <Table
            size="small"
            pagination={{ pageSize: 50, size: "small" }}
            scroll={{ y: 360, x: "max-content" }}
            sticky
            locale={{ emptyText: "无 findings" }}
            columns={findingsColumns}
            dataSource={findingsData}
          />
        </Card>
      </Space>
    </Card>
  );
}
