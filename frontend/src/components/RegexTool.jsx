import React, { useEffect, useMemo, useState } from "react";
import {
  Card,
  Tabs,
  Input,
  Button,
  Table,
  Select,
  Form,
  Alert,
  Empty,
  Typography,
  Space,
  InputNumber,
  Switch,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ExperimentOutlined,
  CopyOutlined,
} from "@ant-design/icons";
import {
  explainRegex,
  generateRegex,
  listRegexTemplates,
} from "../tauri.js";

const { TextArea } = Input;
const { Text, Paragraph } = Typography;

// 正则解析 / 模板生成子界面（v0.4.0 Tools Tab 的「正则解析」页）。
//
// 两个子 Tab：
//   ① 解析：输入正则 → 调 explain_regex → antd Table 显示 token 解释。
//   ② 模板：选模板 → 按 params_schema 填参数 → 调 generate_regex →
//           生成正则 + 测试样例输入框 + 用前端 new RegExp 跑 match 高亮命中片段。
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
// 子组件：模板 Tab
// ─────────────────────────────────────────────────────────────────────
function TemplateTab({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [templates, setTemplates] = useState([]);
  const [loadingTemplates, setLoadingTemplates] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [testInput, setTestInput] = useState("");
  const [testError, setTestError] = useState(null);
  const [testHighlight, setTestHighlight] = useState(null);

  // 拉取模板清单
  useEffect(() => {
    let alive = true;
    setLoadingTemplates(true);
    listRegexTemplates()
      .then((list) => {
        if (!alive) return;
        setTemplates(Array.isArray(list) ? list : []);
      })
      .catch((e) => {
        if (!alive) return;
        message.error(`加载模板清单失败: ${e}`);
      })
      .finally(() => {
        if (!alive) return;
        setLoadingTemplates(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  const selectedName = state.regexTemplateSelected;
  const selectedMeta = useMemo(
    () => templates.find((t) => t.name === selectedName) || null,
    [templates, selectedName]
  );

  // 选中模板时把 params 初始化为 default
  const onSelectTemplate = (name) => {
    dispatch({ type: "SET_REGEX_TEMPLATE_SELECTED", regexTemplateSelected: name });
    const meta = templates.find((t) => t.name === name) || null;
    const init = {};
    (meta?.params_schema || []).forEach((p) => {
      init[p.key] = p.default != null ? p.default : "";
    });
    dispatch({ type: "SET_REGEX_TEMPLATE_PARAMS", regexTemplateParams: init });
    dispatch({ type: "SET_REGEX_GENERATED", regexGenerated: null });
    setTestInput("");
    setTestHighlight(null);
    setTestError(null);
  };

  const onParamChange = (key, value) => {
    const next = { ...state.regexTemplateParams, [key]: value };
    dispatch({ type: "SET_REGEX_TEMPLATE_PARAMS", regexTemplateParams: next });
  };

  const onGenerate = async () => {
    if (!selectedName) {
      message.warning("请先选择模板");
      return;
    }
    setGenerating(true);
    setTestError(null);
    setTestHighlight(null);
    try {
      // 后端 params 为 HashMap<String,String>；number/boolean 统一转 string。
      const params = {};
      for (const [k, v] of Object.entries(state.regexTemplateParams || {})) {
        params[k] = v == null ? "" : String(v);
      }
      const r = await generateRegex(selectedName, params);
      dispatch({ type: "SET_REGEX_GENERATED", regexGenerated: r });
    } catch (e) {
      message.error(`生成失败: ${e}`);
      dispatch({ type: "SET_REGEX_GENERATED", regexGenerated: null });
    } finally {
      setGenerating(false);
    }
  };

  const onTest = () => {
    const pattern = state.regexGenerated;
    if (!pattern) {
      message.warning("请先生成正则");
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
      const nodes = highlightMatches(testInput, re);
      setTestHighlight(nodes);
      if (nodes.length === 0 || (nodes.length === 1 && nodes[0].type === "span")) {
        // 全无命中
      }
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
    <Card title="正则模板生成" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <div>
          <Text type="secondary" style={{ fontSize: 12 }}>
            选择一个预置模板，按参数表单填值后生成正则；可在测试样例输入框中用前端 RegExp 跑 match 查看命中高亮。
          </Text>
        </div>

        <Form layout="vertical" style={{ maxWidth: 720 }}>
          <Form.Item label="模板">
            <Select
              placeholder="选择模板"
              loading={loadingTemplates}
              value={selectedName || undefined}
              onChange={onSelectTemplate}
              options={templates.map((t) => ({
                label: `${t.name} — ${t.description}`,
                value: t.name,
              }))}
              allowClear
              onClear={() =>
                dispatch({
                  type: "SET_REGEX_TEMPLATE_SELECTED",
                  regexTemplateSelected: null,
                })
              }
            />
          </Form.Item>

          {selectedMeta && selectedMeta.params_schema.length > 0 ? (
            selectedMeta.params_schema.map((p) => (
              <Form.Item
                key={p.key}
                label={
                  <Space size={4}>
                    <Text strong>{p.key}</Text>
                    {p.required ? (
                      <Text type="danger" style={{ fontSize: 12 }}>
                        *
                      </Text>
                    ) : null}
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {p.description}
                    </Text>
                  </Space>
                }
              >
                <ParamInput
                  param={p}
                  value={state.regexTemplateParams?.[p.key]}
                  onChange={(v) => onParamChange(p.key, v)}
                />
              </Form.Item>
            ))
          ) : null}

          <Form.Item>
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              loading={generating}
              onClick={onGenerate}
              disabled={!selectedName}
            >
              生成
            </Button>
          </Form.Item>
        </Form>

        {state.regexGenerated != null ? (
          <Card
            title="生成结果"
            size="small"
            styles={{ body: { padding: 12 } }}
            extra={
              <Button
                size="small"
                icon={<CopyOutlined />}
                onClick={() => {
                  navigator.clipboard?.writeText(state.regexGenerated);
                  message.success("已复制");
                }}
              >
                复制
              </Button>
            }
          >
            <Paragraph copyable style={{ fontFamily: "monospace", marginBottom: 12 }}>
              {state.regexGenerated}
            </Paragraph>

            <Space.Compact style={{ width: "100%" }}>
              <Input
                placeholder="测试样例（如 a@b.com）"
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
            description="选择模板并生成后此处显示正则与测试样例"
          />
        )}
      </Space>
    </Card>
  );
}

// 模板参数输入控件：按后端 params_schema 的 kind 渲染。
// 后端 ParamSchema 当前只有 key/description/required/default 四字段，无 kind；
// 按值类型推断（default 是 "true"/"false" → boolean，纯数字 → number，否则 string）。
// HANDOFF 要求最小实现支持 string/number/boolean 三类，这里以 default 值形态推断。
function ParamInput({ param, value, onChange }) {
  const defVal = param.default;
  let kind = "string";
  if (defVal === "true" || defVal === "false") {
    kind = "boolean";
  } else if (defVal != null && /^-?\d+(\.\d+)?$/.test(defVal)) {
    kind = "number";
  }
  switch (kind) {
    case "boolean":
      return (
        <Switch
          checked={value === "true" || value === true}
          onChange={(v) => onChange(v ? "true" : "false")}
        />
      );
    case "number":
      return (
        <InputNumber
          style={{ width: 240 }}
          value={value == null || value === "" ? null : Number(value)}
          onChange={(v) => onChange(v == null ? "" : String(v))}
        />
      );
    case "string":
    default:
      return (
        <Input
          style={{ width: 240 }}
          value={value == null ? "" : String(value)}
          placeholder={param.description}
          onChange={(e) => onChange(e.target.value)}
        />
      );
  }
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
        { key: "template", label: "模板生成", children: <TemplateTab state={state} dispatch={dispatch} /> },
      ]}
    />
  );
}
