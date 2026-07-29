import React, { useMemo, useState } from "react";
import {
  Card,
  Table,
  Button,
  Typography,
  Space,
  Descriptions,
  Empty,
  Spin,
  Input,
  Tag,
  App as AntApp,
} from "antd";
import { PlayCircleOutlined } from "@ant-design/icons";
import { parseSqlTool } from "../tauri.js";

const { Text } = Typography;
const { TextArea } = Input;

// Tools/SQL 解析子界面（v0.4.0 T5-10）：
//   ① Input.TextArea 多行输入 SQL 探针序列（每行一条）
//   ② 「解析」按钮调 parse_sql_tool（T5-9 命令）
//   ③ 上方 Descriptions：schema + 表数 + unmatched 数
//   ④ 每张 ReconstructedTable 一个 inner Card + antd Table（全量列做表头，
//     未 fetch 列 cells 为 null → 显示 `-`）+ unmatched 兜底列表
//   ⑤ 下方探针明细表：read_target / char_position / comparator / compared_ascii / kind / source_ip
//
// 输入是纯 SQL 文本，前端拆行后构造 SqlParseInput 数组。
// v0.7.2：每行支持 `body|sql` 前缀，提取 response_body_size（盲注二分还原必需）。
// 无前缀行 response_body_size / source_ip 留空 None（非盲注 payload 走 parse_payload 兜底）。
export default function SqlParseTool({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [loading, setLoading] = useState(false);
  // v0.6.4 T19-3：分页受控，切 pageSize 立即生效。两张表各一组。
  const [pageCurrent1, setPageCurrent1] = useState(1);
  const [pageSize1, setPageSize1] = useState(50);
  const [pageCurrent2, setPageCurrent2] = useState(1);
  const [pageSize2, setPageSize2] = useState(50);

  const result = state.sqlParseResult;
  const reconstructed = result?.reconstructed || null;
  const probes = result?.probes || [];
  const parsedPayloads = result?.parsed_payloads || [];
  const unmatchedResults = reconstructed?.unmatched_results || [];
  const tables = reconstructed?.tables || [];

  const onParse = async () => {
    const raw = state.sqlParseInput || "";
    const lines = raw
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean);
    if (lines.length === 0) {
      message.warning("请输入 SQL 探针序列");
      return;
    }
    // v0.7.2：每行支持 `body|sql` 前缀，提取 response_body_size（盲注二分
    // 还原必需，probe.rs 无 body_size 会返回空 Vec → 0 探针 → 还原失败）。
    // 无前缀行 responseBodySize=null（向后兼容非盲注 payload：time/error/union 等）。
    const inputs = lines.map((line) => {
      const m = /^(\d+)\|(.*)$/.exec(line);
      if (m) {
        const bodySize = Number(m[1]);
        const sql = m[2];
        return {
          sql,
          responseBodySize: Number.isFinite(bodySize) ? bodySize : null,
          sourceIp: null,
        };
      }
      return { sql: line, responseBodySize: null, sourceIp: null };
    });
    setLoading(true);
    try {
      const res = await parseSqlTool(inputs);
      dispatch({ type: "SET_SQL_PARSE_RESULT", sqlParseResult: res });
    } catch (e) {
      message.error(`解析失败: ${e}`);
      dispatch({ type: "SET_SQL_PARSE_RESULT", sqlParseResult: null });
    } finally {
      setLoading(false);
    }
  };

  const probeColumns = [
    { title: "#", key: "idx", width: 48, render: (_, __, idx) => idx + 1 },
    { title: "read_target", dataIndex: "read_target", key: "read_target", ellipsis: true },
    {
      title: "kind",
      dataIndex: "probe_kind",
      key: "probe_kind",
      width: 120,
      render: (k) => <Tag>{k}</Tag>,
    },
    {
      title: "char_position",
      dataIndex: "char_position",
      key: "char_position",
      width: 110,
      align: "right",
    },
    {
      title: "comparator",
      key: "comparator",
      width: 100,
      render: (_, r) => {
        // blind_boolean 探针无 comparator 字段（ParsedPayload 才有）；这里回填
        // 一个根据 probe_kind 推断的占位：Length/AsciiBinary 都隐含 `=`/`<`/`>`
        // 二分。真实 comparator 在 parsed_payloads 表里展示。这里仅展示
        // equality_char（如有）。
        if (r.equality_char != null) {
          return <Tag color="blue">={r.equality_char}</Tag>;
        }
        return <Text type="secondary">-</Text>;
      },
    },
    {
      title: "threshold",
      dataIndex: "threshold",
      key: "threshold",
      width: 90,
      align: "right",
    },
    { title: "source_ip", dataIndex: "source_ip", key: "source_ip", width: 140 },
    {
      title: "body_size",
      dataIndex: "body_size",
      key: "body_size",
      width: 100,
      align: "right",
    },
  ];

  const probeDataSource = useMemo(
    () => probes.map((p, idx) => ({ ...p, key: idx })),
    [probes]
  );

  return (
    <Card title="SQL 解析" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <div>
          <Text type="secondary" style={{ fontSize: 12 }}>
            每行一条 SQL 探针序列。盲注二分还原需带 response_body_size，格式：
            <code>&lt;body_size&gt;|&lt;sql&gt;</code>（如 862|ascii(substr((database()),1,1))&gt;79）。
            无前缀行视为非盲注 payload（time/error/union/tautology/comment 等，走 parse_payload 兜底）。
          </Text>
        </div>
        <Space.Compact style={{ width: "100%" }}>
          <TextArea
            value={state.sqlParseInput}
            onChange={(e) =>
              dispatch({
                type: "SET_SQL_PARSE_INPUT",
                sqlParseInput: e.target.value,
              })
            }
            placeholder={
              "862|ascii(substr((database()),1,1))>79\n875|ascii(substr((database()),1,1))>112\n862|username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1\n..."
            }
            autoSize={{ minRows: 4, maxRows: 12 }}
            style={{ fontFamily: "monospace" }}
            onPressEnter={(e) => {
              if (e.ctrlKey || e.metaKey) onParse();
            }}
          />
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={loading}
            onClick={onParse}
            style={{ marginLeft: 8 }}
          >
            解析
          </Button>
        </Space.Compact>

        {result ? (
          <>
            <Descriptions
              size="small"
              column={4}
              bordered
              items={[
                {
                  key: "schema",
                  label: "schema",
                  children: reconstructed?.schema || <Text type="secondary">-</Text>,
                },
                {
                  key: "tables",
                  label: "表数",
                  children: <Text strong>{tables.length}</Text>,
                },
                {
                  key: "unmatched",
                  label: "unmatched",
                  children: <Text strong>{unmatchedResults.length}</Text>,
                },
                {
                  key: "probes",
                  label: "探针数",
                  children: <Text strong>{probes.length}</Text>,
                },
              ]}
            />

            <Spin spinning={loading}>
              <Space direction="vertical" size="middle" style={{ width: "100%" }}>
                {tables.length === 0 ? (
                  <Empty
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description="无重建表"
                  />
                ) : (
                  tables.map((tbl, ti) => (
                    <Card
                      key={`tbl-${ti}`}
                      size="small"
                      title={`表: ${tbl.name}`}
                      extra={
                        <Space size={4}>
                          {tbl.column_separator != null && (
                            <Tag color="blue">分隔符 {JSON.stringify(tbl.column_separator)}</Tag>
                          )}
                          <Tag>列 {tbl.columns.length}</Tag>
                          <Tag>行 {tbl.rows.length}</Tag>
                          <Tag>探针 {tbl.source_probe_count}</Tag>
                        </Space>
                      }
                    >
                      <Table
                        size="small"
                        pagination={false}
                        scroll={{ x: "max-content" }}
                        rowKey={(_, ri) => ri}
                        dataSource={tbl.rows.map((r, ri) => ({
                          key: ri,
                          cells: r.cells,
                        }))}
                        columns={tbl.columns.map((col, ci) => ({
                          title: col,
                          key: `col-${ci}`,
                          width: 160,
                          render: (row) => {
                            const v = row.cells?.[ci];
                            return v == null ? (
                              <Text type="secondary">-</Text>
                            ) : (
                              <Text>{v}</Text>
                            );
                          },
                        }))}
                      />
                    </Card>
                  ))
                )}

                {unmatchedResults.length > 0 && (
                  <Card size="small" title="unmatched 兜底">
                    <Table
                      size="small"
                      pagination={false}
                      scroll={{ x: "max-content" }}
                      dataSource={unmatchedResults.map((r, idx) => ({
                        key: idx,
                        read_target: r.read_target,
                        decoded_string: r.decoded_string,
                        probe_count: r.probe_count,
                        source_ips: (r.source_ips || []).join(", "),
                      }))}
                      columns={[
                        { title: "read_target", dataIndex: "read_target", key: "read_target", ellipsis: true },
                        { title: "decoded", dataIndex: "decoded_string", key: "decoded_string" },
                        { title: "probes", dataIndex: "probe_count", key: "probe_count", width: 90, align: "right" },
                        { title: "source_ips", dataIndex: "source_ips", key: "source_ips", width: 160 },
                      ]}
                    />
                  </Card>
                )}

                <Card size="small" title="探针明细">
                  <Table
                    size="small"
                    columns={probeColumns}
                    dataSource={probeDataSource}
                    pagination={{
                      current: pageCurrent1,
                      pageSize: pageSize1,
                      showSizeChanger: true,
                      pageSizeOptions: [10, 20, 50, 100],
                      onChange: (page, ps) => {
                        setPageCurrent1(page);
                        setPageSize1(ps);
                      },
                      onShowSizeChange: (page, ps) => {
                        setPageCurrent1(1);
                        setPageSize1(ps);
                      },
                      showTotal: (t) => `共 ${t} 条`,
                    }}
                    scroll={{ y: 400, x: "max-content" }}
                    locale={{ emptyText: "无探针" }}
                  />
                </Card>

                {parsedPayloads.length > 0 && (
                  <Card size="small" title="payload 语义解析（parsed_payloads）">
                    <Table
                      size="small"
                      dataSource={parsedPayloads.map((p, idx) => ({
                        key: idx,
                        attack_type: p.attack_type,
                        technique: p.technique,
                        read_target: p.read_target ?? "-",
                        char_position: p.char_position ?? "-",
                        comparator: p.comparator ?? "-",
                        compared_ascii: p.compared_ascii ?? "-",
                        union_columns: p.union_columns ?? "-",
                        sleep_seconds: p.sleep_seconds ?? "-",
                        summary: p.summary,
                      }))}
                      pagination={{
                        current: pageCurrent2,
                        pageSize: pageSize2,
                        showSizeChanger: true,
                        pageSizeOptions: [10, 20, 50, 100],
                        onChange: (page, ps) => {
                          setPageCurrent2(page);
                          setPageSize2(ps);
                        },
                        onShowSizeChange: (page, ps) => {
                          setPageCurrent2(1);
                          setPageSize2(ps);
                        },
                        showTotal: (t) => `共 ${t} 条`,
                      }}
                      scroll={{ y: 320, x: "max-content" }}
                      columns={[
                        { title: "attack_type", dataIndex: "attack_type", key: "attack_type", width: 120, render: (t) => <Tag color="volcano">{t}</Tag> },
                        { title: "technique", dataIndex: "technique", key: "technique", width: 140, ellipsis: true },
                        { title: "read_target", dataIndex: "read_target", key: "read_target", ellipsis: true },
                        { title: "char_pos", dataIndex: "char_position", key: "char_position", width: 80, align: "right" },
                        { title: "comparator", dataIndex: "comparator", key: "comparator", width: 90 },
                        { title: "compared_ascii", dataIndex: "compared_ascii", key: "compared_ascii", width: 110, align: "right" },
                        { title: "union_cols", dataIndex: "union_columns", key: "union_columns", width: 90, align: "right" },
                        { title: "sleep_s", dataIndex: "sleep_seconds", key: "sleep_seconds", width: 80, align: "right" },
                        { title: "summary", dataIndex: "summary", key: "summary", ellipsis: true },
                      ]}
                      locale={{ emptyText: "无 payload 语义解析" }}
                    />
                  </Card>
                )}
              </Space>
            </Spin>
          </>
        ) : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description="请输入 SQL 探针序列"
          />
        )}
      </Space>
    </Card>
  );
}
