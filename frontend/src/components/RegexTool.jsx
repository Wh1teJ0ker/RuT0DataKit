import React, { useMemo, useState } from "react";
import {
  Card,
  Tabs,
  Input,
  Button,
  Table,
  Alert,
  Empty,
  Typography,
  Space,
  Tag,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ExperimentOutlined,
  CopyOutlined,
} from "@ant-design/icons";
import { explainRegex, regexConstruct } from "../tauri.js";

const { TextArea } = Input;
const { Text, Paragraph } = Typography;

// 正则解析 / 构造子界面（v0.4.0 Tools Tab 的「正则解析」页）。
//
// 两个子 Tab：
//   ① 解析：输入正则 → 调 explain_regex → antd Table 显示 token 解释。
//   ② 构造（v0.4.1 T6-5）：输入一句自然语言描述 → 调 regex_construct →
//      返回 { pattern, explanation, matched_clues }，前端展示 pattern +
//      matched_clues（Tag list）+ 测试样例高亮（复用 highlightMatches）。
//
// state/dispatch 从 props 透传；子 Tab 选中与各子状态写入全局 state，切 view 不丢。
//
// UX 风险（HANDOFF 要求）：Rust `regex` crate 与浏览器 `new RegExp` 均不支持
// 回溯引用（backref）与零宽断言（look-around）。当解析结果 token 中出现
// kind=backref 或 kind=assertion 时，在解析结果表上方用 antd Alert (warning)
// 提示「仅作教学说明，无法在测试样例中执行」。
const BACKREF_ALERT_TEXT =
  "本地引擎（Rust regex / 浏览器 RegExp）不支持回溯引用/零宽断言，本解析仅作教学说明，无法在测试样例中执行";

// 把后端返回的 (?i) ... 内联标志剥离给 new RegExp 用（JS 支持 (?i:) 但不支持
// 顶层 (?i) 前缀；这里把 (?i) / (?m) / (?s) 顶层前缀转成 flags 形态）。
// 同时返回 { source, flags }，便于 new RegExp(source, flags)。
function adaptRegexForJs(pattern) {
  let flags = "";
  let src = pattern;
  // 处理顶层 (?i) / (?m) / (?s) / (?U) 等开关组：吸收到 flags。
  // 仅识别紧贴开头的 (?flags) 形式；其余 (?i:...) 内联形式 JS 支持，保留。
  const m = /^\(\?([imsU]+)\)/.exec(src);
  if (m) {
    const f = m[1];
    if (f.includes("i")) flags += "i";
    if (f.includes("m")) flags += "m";
    if (f.includes("s")) flags += "s";
    src = src.slice(m[0].length);
  }
  return { source: src, flags };
}

// 把测试样例文本按命中切片，返回 React 节点数组（命中片段用 <mark> 包裹）。
// 不使用 dangerouslySetInnerHTML，规避 XSS。
function highlightMatches(text, regex) {
  if (!text) return [];
  let re;
  try {
    re = new RegExp(regex.source, regex.flags.includes("g") ? regex.flags : regex.flags + "g");
  } catch (e) {
    throw e;
  }
  const out = [];
  let last = 0;
  let m;
  let idx = 0;
  while ((m = re.exec(text)) !== null) {
    if (m.index === last && m[0] === "") {
      // 零宽匹配避免死循环
      re.lastIndex++;
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
  return out;
}

// ─────────────────────────────────────────────────────────────────────
// 子组件：解析 Tab
// ─────────────────────────────────────────────────────────────────────
function ExplainTab({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [loading, setLoading] = useState(false);

  const tokens = state.regexExplainResult || [];
  const hasResult = state.regexExplainResult != null;

  // backref / assertion UX 提示
  const needsBackrefAlert = useMemo(
    () =>
      tokens.some(
        (t) => t.kind === "backref" || t.kind === "assertion"
      ),
    [tokens]
  );

  const onExplain = async () => {
    const p = state.regexExplainInput || "";
    if (!p.trim()) {
      message.warning("请输入正则表达式");
      return;
    }
    setLoading(true);
    try {
      const res = await explainRegex(p);
      dispatch({ type: "SET_REGEX_EXPLAIN_RESULT", regexExplainResult: res });
    } catch (e) {
      message.error(`解析失败: ${e}`);
      dispatch({ type: "SET_REGEX_EXPLAIN_RESULT", regexExplainResult: null });
    } finally {
      setLoading(false);
    }
  };

  const columns = [
    {
      title: "#",
      key: "idx",
      width: 48,
      render: (_, __, idx) => idx + 1,
    },
    {
      title: "token",
      dataIndex: "token",
      key: "token",
      width: 160,
      render: (t) => <Text code copyable={false}>{t}</Text>,
    },
    {
      title: "kind",
      dataIndex: "kind",
      key: "kind",
      width: 120,
      render: (k) => <Text type="secondary">{k}</Text>,
    },
    {
      title: "description",
      dataIndex: "description",
      key: "description",
    },
    {
      title: "position",
      dataIndex: "position",
      key: "position",
      width: 90,
      align: "right",
    },
  ];

  const dataSource = useMemo(
    () => tokens.map((t, idx) => ({ ...t, key: idx })),
    [tokens]
  );

  return (
    <Card title="正则解析" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <div>
          <Text type="secondary" style={{ fontSize: 12 }}>
            输入一条正则表达式，逐 token 解释其含义。Rust regex / 浏览器 RegExp 不支持的语法（回溯引用、零宽断言）会标记为 backref / assertion，仅供教学。
          </Text>
        </div>
        <Space.Compact style={{ width: "100%" }}>
          <TextArea
            value={state.regexExplainInput}
            onChange={(e) =>
              dispatch({
                type: "SET_REGEX_EXPLAIN_INPUT",
                regexExplainInput: e.target.value,
              })
            }
            placeholder="例如：^1[3-9]\d{9}$"
            autoSize={{ minRows: 1, maxRows: 4 }}
            style={{ fontFamily: "monospace" }}
            onPressEnter={(e) => {
              if (e.ctrlKey || e.metaKey) onExplain();
            }}
          />
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={loading}
            onClick={onExplain}
            style={{ marginLeft: 8 }}
          >
            解析
          </Button>
        </Space.Compact>

        {hasResult ? (
          <>
            {needsBackrefAlert ? (
              <Alert
                type="warning"
                showIcon
                message={BACKREF_ALERT_TEXT}
              />
            ) : null}
            <Table
              size="small"
              pagination={false}
              scroll={{ y: 360, x: "max-content" }}
              columns={columns}
              dataSource={dataSource}
              locale={{ emptyText: "无 token" }}
            />
          </>
        ) : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description="输入正则后点击「解析」查看 token 解释"
          />
        )}
      </Space>
    </Card>
  );
}

// ─────────────────────────────────────────────────────────────────────
// 子组件：构造 Tab（v0.4.1 T6-5：自然语言描述 → 正则）
// ─────────────────────────────────────────────────────────────────────
function ConstructTab({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [loading, setLoading] = useState(false);
  const [testInput, setTestInput] = useState("");
  const [testError, setTestError] = useState(null);
  const [testHighlight, setTestHighlight] = useState(null);

  const result = state.regexConstructResult; // { pattern, explanation, matched_clues }

  const onConstruct = async () => {
    const s = state.regexConstructInput || "";
    if (!s.trim()) {
      message.warning("请输入描述语句");
      return;
    }
    setLoading(true);
    setTestError(null);
    setTestHighlight(null);
    try {
      const r = await regexConstruct(s);
      dispatch({ type: "SET_REGEX_CONSTRUCT_RESULT", regexConstructResult: r });
    } catch (e) {
      message.error(`构造失败: ${e}`);
      dispatch({ type: "SET_REGEX_CONSTRUCT_RESULT", regexConstructResult: null });
    } finally {
      setLoading(false);
    }
  };

  const onTest = () => {
    const pattern = result?.pattern;
    if (!pattern) {
      message.warning("请先构造正则");
      return;
    }
    if (!testInput) {
      message.warning("请输入测试样例");
      return;
    }
    setTestError(null);
    try {
      const { source, flags } = adaptRegexForJs(pattern);
      const re = new RegExp(source, flags);
      setTestHighlight(highlightMatches(testInput, re));
    } catch (e) {
      setTestHighlight(null);
      setTestError(`本地 RegExp 执行失败: ${String(e).replace(/^Error: /, "")}`);
    }
  };

  const hasMatch = useMemo(() => {
    if (!testHighlight) return false;
    return testHighlight.some((n) => n.type === "mark");
  }, [testHighlight]);

  return (
    <Card title="正则构造" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Text type="secondary" style={{ fontSize: 12 }}>
          输入一句自然语言描述（如「11 位手机号」「大写字母 8 位」「以 1 开头 11 位」「邮箱」「http 链接」「18 位身份证」），自动构造正则。
        </Text>
        <Space.Compact style={{ width: "100%" }}>
          <TextArea
            value={state.regexConstructInput}
            onChange={(e) =>
              dispatch({
                type: "SET_REGEX_CONSTRUCT_INPUT",
                regexConstructInput: e.target.value,
              })
            }
            placeholder="例如：以 1 开头 11 位"
            autoSize={{ minRows: 1, maxRows: 4 }}
            onPressEnter={(e) => {
              if (e.ctrlKey || e.metaKey) onConstruct();
            }}
          />
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={loading}
            onClick={onConstruct}
            style={{ marginLeft: 8 }}
          >
            构造
          </Button>
        </Space.Compact>

        {result ? (
          <Card
            title="构造结果"
            size="small"
            styles={{ body: { padding: 12 } }}
            extra={
              <Button
                size="small"
                icon={<CopyOutlined />}
                onClick={() => {
                  navigator.clipboard?.writeText(result.pattern);
                  message.success("已复制");
                }}
              >
                复制
              </Button>
            }
          >
            <Paragraph copyable style={{ fontFamily: "monospace", marginBottom: 12 }}>
              {result.pattern}
            </Paragraph>
            <div style={{ marginBottom: 8 }}>
              <Text type="secondary" style={{ fontSize: 12 }}>
                推断依据：
              </Text>
              {result.matched_clues && result.matched_clues.length > 0 ? (
                <Space size={4} wrap style={{ marginLeft: 8 }}>
                  {result.matched_clues.map((c, i) => (
                    <Tag key={i}>{c}</Tag>
                  ))}
                </Space>
              ) : (
                <Text type="secondary" style={{ fontSize: 12 }}>
                  无
                </Text>
              )}
            </div>
            <Space.Compact style={{ width: "100%" }}>
              <Input
                placeholder="测试样例（如 13800138000）"
                value={testInput}
                onChange={(e) => setTestInput(e.target.value)}
                onPressEnter={onTest}
              />
              <Button
                icon={<ExperimentOutlined />}
                onClick={onTest}
                style={{ marginLeft: 8 }}
              >
                测试
              </Button>
            </Space.Compact>
            {testError ? (
              <Alert
                type="error"
                showIcon
                message={testError}
                style={{ marginTop: 8 }}
              />
            ) : null}
            {testHighlight != null && !testError ? (
              <div
                style={{
                  marginTop: 8,
                  padding: 8,
                  background: "#fafafa",
                  border: "1px solid #f0f0f0",
                  borderRadius: 4,
                  wordBreak: "break-all",
                }}
              >
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {hasMatch ? "命中片段高亮：" : "未命中（无匹配片段）"}
                </Text>
                <div style={{ marginTop: 4, lineHeight: 1.8 }}>
                  {testHighlight.length > 0 ? (
                    testHighlight
                  ) : (
                    <Text type="secondary">{testInput}</Text>
                  )}
                </div>
              </div>
            ) : null}
          </Card>
        ) : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description="输入描述语句后点击「构造」"
          />
        )}
      </Space>
    </Card>
  );
}

export default function RegexTool({ state, dispatch }) {
  return (
    <Tabs
      activeKey={state.regexSubTab}
      onChange={(k) =>
        dispatch({ type: "SET_REGEX_SUB_TAB", regexSubTab: k })
      }
      items={[
        { key: "explain", label: "解析", children: <ExplainTab state={state} dispatch={dispatch} /> },
        { key: "construct", label: "构造", children: <ConstructTab state={state} dispatch={dispatch} /> },
      ]}
    />
  );
}
