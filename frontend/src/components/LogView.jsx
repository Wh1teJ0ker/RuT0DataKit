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
import { tauriInvoke, scanLogFile } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";

const { Text } = Typography;

// 日志扫描视图（v0.2.0）：四段垂直布局，与 MaskView/ValidateView 同模式。
//   ① 顶部：导入 .log 按钮（select_file）+ 文件路径显示
//   ② 原始日志表（state.logEntries.slice(0,200)，分页 pageSize=50）
//   ③ 「运行扫描」按钮 + summary 卡片
//   ④ findings 表（分页 pageSize=50）
// state/dispatch 从 props 透传；logEntries/logReport/logLoading 在全局
// useReducer state，切 view 不丢数据。
export default function LogView({ state, dispatch }) {
  const { message } = AntApp.useApp();

  // 导入 .log：select_file 拿路径 → 直接调 scanLogFile 拿 {entries, report}。
  // 不复用 FileToolbar 的 load_preview（csv/xlsx 专用）。
  const handleSelectFile = async () => {
    dispatch({ type: "SET_LOG_LOADING", logLoading: true });
    try {
      const chosen = await tauriInvoke("select_file");
      if (!chosen) {
        dispatch({ type: "SET_LOG_LOADING", logLoading: false });
        return;
      }
      // 用 detect_source_type 校验是否为 log 类型。
      try {
        const t = await tauriInvoke("detect_source_type", { path: chosen });
        if (t !== "log") {
          message.warning(`日志扫描仅支持 .log 文件，当前识别为 ${t}`);
          dispatch({ type: "SET_LOG_LOADING", logLoading: false });
          return;
        }
      } catch (e) {
        message.error(`类型识别失败: ${e}`);
        dispatch({ type: "SET_LOG_LOADING", logLoading: false });
        return;
      }
      try {
        const res = await scanLogFile(chosen);
        dispatch({
          type: "SET_FILE",
          filePath: chosen,
          sourceType: "log",
          headers: [],
          rows: [],
          rowCount: (res.entries || []).length,
        });
        dispatch({ type: "SET_LOG_ENTRIES", logEntries: res.entries || [] });
        dispatch({ type: "SET_LOG_REPORT", logReport: res.report || null });
        dispatch({ type: "SET_HINT", actionHint: "" });
      } catch (e) {
        message.error(`日志扫描失败: ${e}`);
      }
    } catch (e) {
      message.error(String(e));
    } finally {
      dispatch({ type: "SET_LOG_LOADING", logLoading: false });
    }
  };

  // 段 ①/② 原始日志表：slice 前 200 行，分页 50。
  const rawData = useMemo(() => {
    return (state.logEntries || [])
      .slice(0, PREVIEW_ROW_LIMIT)
      .map((e) => ({ ...e, key: e.line_no }));
  }, [state.logEntries]);

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
    { title: "行号", dataIndex: "line_no", width: 70, fixed: "left" },
    { title: "IP", dataIndex: "ip", width: 140, ellipsis: true },
    { title: "Method", dataIndex: "method", width: 80 },
    {
      title: "Path",
      dataIndex: "path",
      ellipsis: true,
      render: (v) => truncateCell(v, 40),
    },
    {
      title: "解码Path",
      dataIndex: "decoded_path",
      ellipsis: true,
      render: (v) => (v ? truncateCell(v, 40) : "-"),
    },
    { title: "Status", dataIndex: "status", width: 70 },
    {
      title: "解码Query",
      dataIndex: "decoded_query",
      ellipsis: true,
      render: (v) =>
        v ? (
          <Tooltip title={v}>
            <span>
              {String(v).length > 60
                ? String(v).slice(0, 60) + "..."
                : v}
            </span>
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
    {
      title: "解码UA",
      dataIndex: "decoded_ua",
      ellipsis: true,
      render: (v) => (v ? truncateCell(v, 40) : "-"),
    },
  ];

  // 段 ③ 重新运行扫描：filePath 已在 state，直接调 scanLogFile。
  const onRunScan = async () => {
    if (!state.filePath) {
      message.warning("请先导入 .log 文件");
      return;
    }
    dispatch({ type: "SET_LOG_LOADING", logLoading: true });
    dispatch({ type: "SET_HINT", actionHint: "正在扫描..." });
    try {
      const res = await scanLogFile(state.filePath);
      dispatch({ type: "SET_LOG_ENTRIES", logEntries: res.entries || [] });
      dispatch({ type: "SET_LOG_REPORT", logReport: res.report || null });
      dispatch({ type: "SET_HINT", actionHint: "扫描完成" });
    } catch (e) {
      message.error(`扫描失败: ${e}`);
      dispatch({ type: "SET_HINT", actionHint: `扫描失败: ${e}` });
    } finally {
      dispatch({ type: "SET_LOG_LOADING", logLoading: false });
    }
  };

  // summary 取自 report.summary（serde_yml::Mapping → JSON 对象）。
  const summary = state.logReport?.summary || null;
  const topAttackIps = summary?.top_attack_ips || [];
  // 段 ③.5 盲注聚合结果：来自 T2-13 report.extra.blind_aggregation。
  const blindAggregation = state.logReport?.extra?.blind_aggregation || [];

  // 段 ④ findings 表：分页 50。
  const findingsData = useMemo(() => {
    return (state.logReport?.findings || []).map((f, idx) => ({
      ...f,
      key: idx,
    }));
  }, [state.logReport]);

  const TAG_COLOR_BY_TYPE = {
    sqli: "red",
    weak_password: "orange",
    idcard: "purple",
    phone: "blue",
    bankcard: "magenta",
  };
  const tagColor = (t) => TAG_COLOR_BY_TYPE[t] || "default";

  // extra.attack_type → 读取目标 Tag 颜色。
  const TAG_COLOR_BY_ATTACK = {
    blind_boolean: "red",
    union: "orange",
    error: "magenta",
    time: "blue",
    tautology: "default",
    comment: "default",
  };

  // 从 f.extra 取字段拼结构化读取目标标签。
  const renderReadTarget = (extra) => {
    if (!extra) return "-";
    const t = extra.attack_type;
    if (!t) return "-";
    let label = null;
    if (t === "blind_boolean") {
      label = `${extra.read_target ?? "-"} · 第${
        extra.char_position ?? "-"
      }字符 · ${extra.comparator ?? ""}${extra.compared_ascii ?? "-"}`;
    } else if (t === "union") {
      label = `UNION ${extra.union_columns ?? "-"}列`;
    } else if (t === "error") {
      label = `读取 ${extra.read_target ?? "-"}`;
    } else if (t === "time") {
      label = `延时 ${extra.sleep_seconds ?? "-"}秒`;
    } else if (t === "tautology" || t === "comment") {
      return "-";
    } else {
      return "-";
    }
    return <Tag color={TAG_COLOR_BY_ATTACK[t] || "default"}>{label}</Tag>;
  };

  const findingsColumns = [
    {
      title: "类型",
      dataIndex: "type",
      width: 120,
      render: (t) => <Tag color={tagColor(t)}>{t}</Tag>,
    },
    {
      title: "值",
      dataIndex: "value",
      ellipsis: true,
      render: (v) => truncateCell(v, 80),
    },
    {
      title: "位置（行号）",
      dataIndex: "location",
      width: 110,
      render: (v) => v ?? "-",
    },
    {
      title: "上下文",
      dataIndex: "context",
      ellipsis: true,
      render: (v) => (v ? truncateCell(v, 80) : "-"),
    },
    {
      title: "解析结果",
      dataIndex: "extra",
      ellipsis: true,
      render: (extra, f) => {
        const v = extra?.summary ?? f?.context;
        return v ? truncateCell(v, 80) : "-";
      },
    },
    {
      title: "读取目标",
      dataIndex: "extra",
      width: 200,
      render: (extra) => renderReadTarget(extra),
    },
  ];

  return (
    <Card title="日志扫描" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {/* 段 ① 导入按钮 + 文件路径 */}
        <Card title="导入日志" styles={{ body: { padding: 12 } }}>
          <Space size="middle" align="center" wrap>
            <Button
              type="primary"
              icon={<UploadOutlined />}
              loading={state.logLoading}
              onClick={handleSelectFile}
            >
              导入 .log
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
            {state.sourceType === "log" && (
              <Tag color="geekblue">类型: log</Tag>
            )}
          </Space>
        </Card>

        {/* 段 ② 原始日志表 */}
        <Card
          title="原始日志"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              共 {state.logEntries.length} 行 / 显示前{" "}
              {Math.min(state.logEntries.length, PREVIEW_ROW_LIMIT)} 行
            </Text>
          }
        >
          <Table
            size="small"
            pagination={{ pageSize: 50, size: "small" }}
            scroll={{ y: 280, x: "max-content" }}
            sticky
            locale={{ emptyText: "导入 .log 后此处显示原始日志" }}
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
              loading={state.logLoading}
              disabled={state.logEntries.length === 0}
              onClick={onRunScan}
            >
              运行扫描
            </Button>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {state.actionHint}
            </Text>
          </Space>
          <Spin spinning={state.logLoading}>
            {state.logReport ? (
              <Descriptions
                size="small"
                bordered
                column={4}
                items={[
                  {
                    key: "total_lines",
                    label: "总行数",
                    children: summary?.total_lines ?? "-",
                  },
                  {
                    key: "sqli_hits",
                    label: "SQLi 命中",
                    children: summary?.sqli_hits ?? 0,
                  },
                  {
                    key: "weak_password_hits",
                    label: "弱口令命中",
                    children: summary?.weak_password_hits ?? 0,
                  },
                  {
                    key: "sensitive_hits",
                    label: "敏感字段命中",
                    children: summary?.sensitive_hits ?? 0,
                  },
                  {
                    key: "top_attack_ips",
                    label: "攻击 IP TOP5",
                    span: 4,
                    children:
                      topAttackIps.length > 0 ? (
                        <Space size="small" wrap>
                          {topAttackIps.map((it, idx) => (
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

        {/* 段 ③.5 盲注聚合结果 */}
        <Card
          title="盲注聚合结果"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              共 {blindAggregation.length} 组
            </Text>
          }
        >
          {blindAggregation.length === 0 ? (
            <Empty description="无盲注二分序列可聚合" />
          ) : (
            <Space direction="vertical" size="small" style={{ width: "100%" }}>
              {blindAggregation.map((r, idx) => (
                <Card
                  key={idx}
                  size="small"
                  type="inner"
                  title={<Text code>{r.read_target}</Text>}
                  extra={
                    <Tag color="red">
                      {r.resolved_chars}/{r.resolved_chars + r.unresolved_chars}{" "}
                      已解
                    </Tag>
                  }
                >
                  <Descriptions size="small" column={2}>
                    <Descriptions.Item label="还原结果">
                      <Text strong copyable>
                        {r.decoded_string || "(空)"}
                      </Text>
                    </Descriptions.Item>
                    <Descriptions.Item label="探针数">
                      <Text>{r.probe_count}</Text>
                    </Descriptions.Item>
                    <Descriptions.Item label="来源 IP">
                      {r.source_ips && r.source_ips.length > 0 ? (
                        <Space size="small" wrap>
                          {r.source_ips.map((ip) => (
                            <Tag key={ip}>{ip}</Tag>
                          ))}
                        </Space>
                      ) : (
                        <Text type="secondary">无</Text>
                      )}
                    </Descriptions.Item>
                    <Descriptions.Item label="位置明细">
                      <Tooltip
                        title={JSON.stringify(r.position_details, null, 2)}
                      >
                        <Text type="secondary">
                          {r.resolved_chars} 已解 / {r.beyond_end_positions}{" "}
                          越界 / {r.unresolved_chars} 未解
                        </Text>
                      </Tooltip>
                    </Descriptions.Item>
                  </Descriptions>
                </Card>
              ))}
            </Space>
          )}
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
