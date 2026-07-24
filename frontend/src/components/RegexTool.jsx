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
  App as AntApp,
} from "antd";
import { PlayCircleOutlined } from "@ant-design/icons";
import { explainRegex } from "../tauri.js";
import RegexConstructTab from "./RegexConstructTab.jsx";

const { TextArea } = Input;
const { Text } = Typography;

// 正则解析 / 构造子界面（v0.4.0 Tools Tab 的「正则解析」页）。
//
// 两个子 Tab：
//   ① 解析：输入正则 → 调 explain_regex → antd Table 显示 token 解释。
//   ② 构造（v0.6.3 T18-2 重构）：可视化积木构建，用户点选「数字」「字母」
//      「至少一次」等模块按顺序拼装正则字符串，实时预览 + 测试样例高亮。
//      纯客户端拼装，不调用后端、不外发数据（与 §6 安全约束一致）。
//
// state/dispatch 从 props 透传；解析子 Tab 选中与各子状态写入全局 state，
// 切 view 不丢。构造子 Tab 状态自包含于组件内部（积木序列、测试样例），
// 不污染全局 state。
//
// UX 风险（HANDOFF 要求）：Rust `regex` crate 与浏览器 `new RegExp` 均不支持
// 回溯引用（backref）与零宽断言（look-around）。当解析结果 token 中出现
// kind=backref 或 kind=assertion 时，在解析结果表上方用 antd Alert (warning)
// 提示「仅作教学说明，无法在测试样例中执行」。
const BACKREF_ALERT_TEXT =
  "本地引擎（Rust regex / 浏览器 RegExp）不支持回溯引用/零宽断言，本解析仅作教学说明，无法在测试样例中执行";

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

export default function RegexTool({ state, dispatch }) {
  return (
    <Tabs
      activeKey={state.regexSubTab}
      onChange={(k) =>
        dispatch({ type: "SET_REGEX_SUB_TAB", regexSubTab: k })
      }
      items={[
        { key: "explain", label: "解析", children: <ExplainTab state={state} dispatch={dispatch} /> },
        { key: "construct", label: "构造", children: <RegexConstructTab /> },
      ]}
    />
  );
}
