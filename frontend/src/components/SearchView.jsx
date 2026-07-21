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
  Radio,
  Input,
  App as AntApp,
  Tag,
} from "antd";
import {
  SearchOutlined,
  SafetyCertificateOutlined,
  ExportOutlined,
} from "@ant-design/icons";
import { searchRecords } from "../tauri.js";

const { Text } = Typography;
const { TextArea, Search: AntSearch } = Input;

// 搜索主视图（v0.4.0 T5-6）：
//   ① 顶部 Radio 三选（关键词 / 正则 / 精确字段）+ 输入区
//   ② 「搜索」按钮调 search_records（T5-5 命令），异步 + loading
//   ③ 顶部 Descriptions 显示命中数 / 耗时
//   ④ 结果表：行号 / 列名 / 字段值 / 片段（命中关键词高亮，用 <mark> React 组件）
//   ⑤ 跳转按钮：把命中行号集合写入 state.filteredRowIndices，dispatch SET_VIEW 跳脱敏/导出
//
// 数据源：state.records（T5-3 预处理产物）。无 records 显示 Empty。
// 片段高亮不用 dangerouslySetInnerHTML——snippet 是后端 ±20 字符上下文，
// 命中关键词在前端用 React <mark> 包裹（escape HTML 自动由 React 处理）。
export default function SearchView({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [loading, setLoading] = useState(false);

  const records = state.records;
  const hasRecords = records != null && Array.isArray(records.headers) && records.headers.length > 0;

  const mode = state.searchMode || "keyword";

  // 构造 SearchQuery JSON（serde tag=kind, snake_case）
  const buildQueryJson = () => {
    if (mode === "keyword") {
      const raw = (state.searchKeywordInput || "").trim();
      if (!raw) return null;
      const terms = raw.split(/\s+/).filter(Boolean);
      const m = (state.searchKeywordMode || "and").toLowerCase();
      return JSON.stringify({ kind: "keyword", terms, mode: m });
    }
    if (mode === "regex") {
      const pattern = (state.searchRegexInput || "").trim();
      if (!pattern) return null;
      return JSON.stringify({ kind: "regex", pattern });
    }
    if (mode === "exact_field") {
      const field = (state.searchExactField || "").trim();
      const value = state.searchExactValue || "";
      if (!field) return null;
      return JSON.stringify({ kind: "exact_field", field, value });
    }
    return null;
  };

  const onSearch = async () => {
    if (!hasRecords) {
      message.warning("请先到数据预处理导入文件");
      return;
    }
    const queryJson = buildQueryJson();
    if (!queryJson) {
      message.warning("请填写查询条件");
      return;
    }
    setLoading(true);
    const t0 = performance.now();
    try {
      const res = await searchRecords(records.headers, records.rows, queryJson);
      const elapsed = Math.round(performance.now() - t0);
      dispatch({ type: "SET_SEARCH_RESULTS", searchResults: res, searchElapsedMs: elapsed });
    } catch (e) {
      message.error(`搜索失败: ${e}`);
      dispatch({ type: "SET_SEARCH_RESULTS", searchResults: null, searchElapsedMs: null });
    } finally {
      setLoading(false);
    }
  };

  const hits = (state.searchResults && state.searchResults.hits) || [];
  const elapsed = state.searchElapsedMs;

  // 跳转：把命中行号去重排序写入 state.filteredRowIndices，dispatch SET_VIEW。
  const jumpTo = (view) => {
    const indices = Array.from(new Set(hits.map((h) => h.row))).sort((a, b) => a - b);
    dispatch({ type: "SET_FILTERED_ROW_INDICES", filteredRowIndices: indices });
    dispatch({ type: "SET_VIEW", activeView: view });
  };

  // 片段高亮：根据 mode 抽取 needle，在 snippet 里用 <mark> 包裹。
  // 不用 dangerouslySetInnerHTML——React 自动 escape 文本节点。
  const highlightSnippet = (snippet) => {
    if (!snippet) return <Text type="secondary">-</Text>;
    let needles = [];
    if (mode === "keyword") {
      needles = (state.searchKeywordInput || "").trim().split(/\s+/).filter(Boolean);
    } else if (mode === "regex") {
      // 正则模式：用生成 RegExp match 包裹命中片段（防非法正则 try/catch）。
      const p = (state.searchRegexInput || "").trim();
      if (p) {
        try {
          const re = new RegExp(p, "g");
          return highlightByRegex(snippet, re);
        } catch {
          // 非法正则——退化为原文展示
          return <Text>{snippet}</Text>;
        }
      }
      return <Text>{snippet}</Text>;
    } else if (mode === "exact_field") {
      const v = state.searchExactValue;
      if (v) needles = [v];
    }
    if (needles.length === 0) return <Text>{snippet}</Text>;
    return highlightByKeywords(snippet, needles);
  };

  const columns = [
    {
      title: "#",
      key: "idx",
      width: 48,
      render: (_, __, idx) => idx + 1,
    },
    { title: "行号", dataIndex: "row", key: "row", width: 80 },
    { title: "列名", dataIndex: "field", key: "field", width: 160 },
    { title: "字段值", dataIndex: "value", key: "value", ellipsis: true },
    {
      title: "片段",
      dataIndex: "snippet",
      key: "snippet",
      render: (s) => highlightSnippet(s),
    },
  ];

  const dataSource = useMemo(
    () => hits.map((h, idx) => ({ ...h, key: idx })),
    [hits]
  );

  // 输入区按 mode 切换
  const renderInputArea = () => {
    if (mode === "keyword") {
      return (
        <Space.Compact style={{ width: "100%" }}>
          <AntSearch
            placeholder="多关键词空格分隔，如 alice 13800"
            value={state.searchKeywordInput}
            onChange={(e) =>
              dispatch({
                type: "SET_SEARCH_KEYWORD_INPUT",
                searchKeywordInput: e.target.value,
              })
            }
            enterButton="搜索"
            onSearch={onSearch}
            loading={loading}
            style={{ flex: 1 }}
          />
          <Radio.Group
            value={state.searchKeywordMode || "and"}
            onChange={(e) =>
              dispatch({
                type: "SET_SEARCH_KEYWORD_MODE",
                searchKeywordMode: e.target.value,
              })
            }
            style={{ marginLeft: 8 }}
            optionType="button"
            buttonStyle="solid"
            options={[
              { value: "and", label: "AND" },
              { value: "or", label: "OR" },
            ]}
          />
        </Space.Compact>
      );
    }
    if (mode === "regex") {
      return (
        <Space.Compact style={{ width: "100%" }}>
          <Input
            placeholder="正则表达式，如 ^1[3-9]\\d{9}$"
            value={state.searchRegexInput}
            onChange={(e) =>
              dispatch({
                type: "SET_SEARCH_REGEX_INPUT",
                searchRegexInput: e.target.value,
              })
            }
            onPressEnter={onSearch}
            style={{ flex: 1, fontFamily: "monospace" }}
          />
          <Button
            type="primary"
            icon={<SearchOutlined />}
            loading={loading}
            onClick={onSearch}
            style={{ marginLeft: 8 }}
          >
            搜索
          </Button>
        </Space.Compact>
      );
    }
    // exact_field
    return (
      <Space.Compact style={{ width: "100%" }}>
        <Input
          placeholder="字段名（如 phone）"
          value={state.searchExactField}
          onChange={(e) =>
            dispatch({
              type: "SET_SEARCH_EXACT_FIELD",
              searchExactField: e.target.value,
            })
            }
          style={{ width: "30%" }}
        />
        <Input
          placeholder="字段值（如 13800138000）"
          value={state.searchExactValue}
          onChange={(e) =>
            dispatch({
              type: "SET_SEARCH_EXACT_VALUE",
              searchExactValue: e.target.value,
            })
          }
          onPressEnter={onSearch}
          style={{ flex: 1 }}
        />
        <Button
          type="primary"
          icon={<SearchOutlined />}
          loading={loading}
          onClick={onSearch}
          style={{ marginLeft: 8 }}
        >
          搜索
        </Button>
      </Space.Compact>
    );
  };

  return (
    <Card title="搜索" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {!hasRecords ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description="请先到数据预处理导入文件"
          />
        ) : (
          <>
            <Radio.Group
              value={mode}
              onChange={(e) =>
                dispatch({ type: "SET_SEARCH_MODE", searchMode: e.target.value })
              }
              optionType="button"
              buttonStyle="solid"
              options={[
                { value: "keyword", label: "关键词" },
                { value: "regex", label: "正则" },
                { value: "exact_field", label: "精确字段" },
              ]}
            />
            {renderInputArea()}

            <Descriptions
              size="small"
              column={3}
              bordered
              items={[
                {
                  key: "hits",
                  label: "命中数",
                  children: <Text strong>{hits.length}</Text>,
                },
                {
                  key: "elapsed",
                  label: "耗时",
                  children: elapsed != null ? `${elapsed} ms` : "-",
                },
                {
                  key: "mode",
                  label: "模式",
                  children: <Tag>{mode}</Tag>,
                },
              ]}
            />

            <Space>
              <Button
                icon={<SafetyCertificateOutlined />}
                onClick={() => jumpTo("mask")}
                disabled={hits.length === 0}
              >
                跳转到数据脱敏
              </Button>
              <Button
                icon={<ExportOutlined />}
                onClick={() => jumpTo("export")}
                disabled={hits.length === 0}
              >
                跳转到数据导出
              </Button>
            </Space>

            <Spin spinning={loading}>
              <Table
                size="small"
                columns={columns}
                dataSource={dataSource}
                pagination={{ pageSize: 50, showSizeChanger: true }}
                scroll={{ y: 400, x: "max-content" }}
                locale={{ emptyText: "无命中或尚未查询" }}
              />
            </Spin>
          </>
        )}
      </Space>
    </Card>
  );
}

// ─────────────────────────────────────────────────────────────────────
// 片段高亮辅助（不使用 dangerouslySetInnerHTML，规避 XSS）。
// ─────────────────────────────────────────────────────────────────────

// 按 keywords 命中切片：大小写不敏感、并行匹配所有 needle。
function highlightByKeywords(text, needles) {
  if (!needles || needles.length === 0) return [<span key="0">{text}</span>];
  const escaped = needles.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const re = new RegExp(`(${escaped.join("|")})`, "gi");
  return splitByRegex(text, re);
}

// 按 regex 命中切片。
function highlightByRegex(text, re) {
  return splitByRegex(text, re);
}

function splitByRegex(text, re) {
  const out = [];
  let last = 0;
  let m;
  let idx = 0;
  // 全局 flag 兜底
  const globalRe = re.global ? re : new RegExp(re.source, (re.flags || "") + "g");
  while ((m = globalRe.exec(text)) !== null) {
    if (m.index === last && m[0] === "") {
      globalRe.lastIndex++;
      continue;
    }
    if (m.index > last) {
      out.push(<span key={`t-${idx++}`}>{text.slice(last, m.index)}</span>);
    }
    out.push(<mark key={`m-${idx++}`}>{m[0]}</mark>);
    last = m.index + m[0].length;
  }
  if (last < text.length) {
    out.push(<span key={`t-${idx++}`}>{text.slice(last)}</span>);
  }
  return <>{out}</>;
}
