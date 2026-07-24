import React, { useMemo, useState, useCallback } from "react";
import {
  Card,
  Input,
  Button,
  Alert,
  Empty,
  Typography,
  Space,
  Tag,
  Tooltip,
  Modal,
  InputNumber,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ExperimentOutlined,
  CopyOutlined,
  UndoOutlined,
  ClearOutlined,
  PlusOutlined,
} from "@ant-design/icons";

const { TextArea } = Input;
const { Text, Paragraph } = Typography;

// ─────────────────────────────────────────────────────────────────────
// 正则可视化积木构造（v0.6.3 T18-2）
// ─────────────────────────────────────────────────────────────────────
//
// 用户点选「数字」「字母」「至少一次」等积木模块，前端按顺序拼装出正则
// 字符串，实时预览 + 测试样例高亮。纯客户端拼装，不调用后端、不外发数据
// （与 docs/00-需求文档.md §6 安全约束一致）。
//
// 积木分 5 类：
//   ① 字符类：数字 / 字母 / 大写 / 小写 / 十六进制 / 任意 / 空白 / 单词字符 ...
//   ② 量词：至少一次 + / 零或多次 * / 可选 ? / 精确 N 次 {n} / 范围 {m,n}
//   ③ 锚定：行首 ^ / 行尾 $
//   ④ 分组/选择：分组 ( ) / 或 |
//   ⑤ 字面量：自定义字面量（自动转义元字符）/ 自定义字符集 [...]
//
// 量词直接追加到序列末尾（作用于前一个 token，用户自行保证顺序）。
//
// 同时提供 3 个预设模板（手机号 / 邮箱 / 身份证）一键载入，便于快速上手。

// 积木模块定义。id 用于 React key；label 是按钮文案；regex 是拼入正则的片段；
// cat 是分类（用于分组渲染）；param 标记需要弹窗收集参数的模块。
const BLOCK_MODULES = [
  // 字符类
  { mid: "digit", label: "数字 \\d", regex: "\\d", cat: "char" },
  { mid: "ndigit", label: "非数字 \\D", regex: "\\D", cat: "char" },
  { mid: "word", label: "单词字符 \\w", regex: "\\w", cat: "char" },
  { mid: "nword", label: "非单词字符 \\W", regex: "\\W", cat: "char" },
  { mid: "space", label: "空白 \\s", regex: "\\s", cat: "char" },
  { mid: "nspace", label: "非空白 \\S", regex: "\\S", cat: "char" },
  { mid: "letter", label: "字母 A-Za-z", regex: "[A-Za-z]", cat: "char" },
  { mid: "upper", label: "大写字母 A-Z", regex: "[A-Z]", cat: "char" },
  { mid: "lower", label: "小写字母 a-z", regex: "[a-z]", cat: "char" },
  { mid: "hex", label: "十六进制 0-9a-fA-F", regex: "[0-9a-fA-F]", cat: "char" },
  { mid: "any", label: "任意字符 .", regex: ".", cat: "char" },
  { mid: "charset", label: "自定义字符集 […]", regex: "", cat: "char", param: "charset" },
  // 量词
  { mid: "plus", label: "至少一次 +", regex: "+", cat: "quant" },
  { mid: "star", label: "零或多次 *", regex: "*", cat: "quant" },
  { mid: "opt", label: "可选 ?", regex: "?", cat: "quant" },
  { mid: "countn", label: "精确 N 次 {n}", regex: "", cat: "quant", param: "count_n" },
  { mid: "countrange", label: "范围 {m,n}", regex: "", cat: "quant", param: "count_range" },
  // 锚定
  { mid: "bol", label: "行首 ^", regex: "^", cat: "anchor" },
  { mid: "eol", label: "行尾 $", regex: "$", cat: "anchor" },
  // 分组/选择
  { mid: "group", label: "分组 ( )", regex: "()", cat: "group" },
  { mid: "alt", label: "或 |", regex: "|", cat: "group" },
  // 字面量
  { mid: "literal", label: "自定义字面量", regex: "", cat: "literal", param: "literal" },
];

const CATEGORY_META = [
  { key: "char", title: "字符类" },
  { key: "quant", title: "量词" },
  { key: "anchor", title: "锚定" },
  { key: "group", title: "分组 / 选择" },
  { key: "literal", title: "字面量" },
];

// 预设模板：一键载入一组积木。展示可视化积木的典型用法。
const PRESETS = [
  {
    key: "phone",
    name: "11 位手机号",
    blocks: [
      { mid: "bol", label: "行首 ^", regex: "^" },
      { mid: "literal", label: "字面量 1", regex: "1" },
      { mid: "charset", label: "字符集 [3-9]", regex: "[3-9]" },
      { mid: "digit", label: "数字 \\d", regex: "\\d" },
      { mid: "countn", label: "精确 9 次 {9}", regex: "{9}" },
      { mid: "eol", label: "行尾 $", regex: "$" },
    ],
  },
  {
    key: "email",
    name: "邮箱",
    blocks: [
      { mid: "bol", label: "行首 ^", regex: "^" },
      { mid: "charset", label: "字符集 [A-Za-z0-9._%+-]", regex: "[A-Za-z0-9._%+-]" },
      { mid: "plus", label: "至少一次 +", regex: "+" },
      { mid: "literal", label: "字面量 @", regex: "@" },
      { mid: "charset", label: "字符集 [A-Za-z0-9.-]", regex: "[A-Za-z0-9.-]" },
      { mid: "plus", label: "至少一次 +", regex: "+" },
      { mid: "literal", label: "字面量 .", regex: "\\." },
      { mid: "letter", label: "字母 A-Za-z", regex: "[A-Za-z]" },
      { mid: "countrange", label: "范围 {2,}", regex: "{2,}" },
      { mid: "eol", label: "行尾 $", regex: "$" },
    ],
  },
  {
    key: "idcard",
    name: "18 位身份证",
    blocks: [
      { mid: "bol", label: "行首 ^", regex: "^" },
      { mid: "digit", label: "数字 \\d", regex: "\\d" },
      { mid: "countn", label: "精确 17 次 {17}", regex: "{17}" },
      { mid: "charset", label: "字符集 [0-9Xx]", regex: "[\\dXx]" },
      { mid: "eol", label: "行尾 $", regex: "$" },
    ],
  },
];

// 把测试样例文本按命中切片，返回 React 节点数组（命中片段用 <mark> 包裹）。
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

// 转义正则元字符，用于「自定义字面量」模块。
function escapeRegexLiteral(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

let blockSeq = 0;

function ConstructTab() {
  const { message } = AntApp.useApp();
  // 积木序列：每项 { key, mid, label, regex }。regex 拼接即得最终 pattern。
  const [blocks, setBlocks] = useState([]);
  // 弹窗参数收集：null 或 { param, value, placeholder, title }
  const [modal, setModal] = useState(null);
  const [testInput, setTestInput] = useState("");
  const [testError, setTestError] = useState(null);
  const [testHighlight, setTestHighlight] = useState(null);

  const pattern = useMemo(() => blocks.map((b) => b.regex).join(""), [blocks]);

  // 推入一个已确定 regex 片段的积木。
  const pushBlock = useCallback((mod, regexOverride, labelOverride) => {
    setBlocks((prev) => [
      ...prev,
      {
        key: `b${blockSeq++}`,
        mid: mod.mid,
        label: labelOverride || mod.label,
        regex: regexOverride != null ? regexOverride : mod.regex,
      },
    ]);
  }, []);

  // 点击模块按钮：无参数模块直接推入；参数模块打开弹窗。
  const onModuleClick = useCallback(
    (mod) => {
      if (!mod.param) {
        pushBlock(mod);
        setTestError(null);
        setTestHighlight(null);
        return;
      }
      // 参数模块：打开弹窗收集参数
      if (mod.param === "count_n") {
        setModal({ param: mod.param, mod, title: "精确 N 次", value: "", placeholder: "输入 N，例如 9", kind: "number" });
      } else if (mod.param === "count_range") {
        setModal({ param: mod.param, mod, title: "范围 {m,n}", value: "", placeholder: "输入 m,n，例如 2,（留空 n 表示至少 m 次）", kind: "text" });
      } else if (mod.param === "literal") {
        setModal({ param: mod.param, mod, title: "自定义字面量", value: "", placeholder: "输入字面文本（元字符自动转义），例如 138", kind: "text" });
      } else if (mod.param === "charset") {
        setModal({ param: mod.param, mod, title: "自定义字符集", value: "", placeholder: "输入字符集内容（不含方括号），例如 3-9 或 A-Z0-9", kind: "text" });
      }
    },
    [pushBlock]
  );

  // 弹窗确认：根据参数类型构造 regex 片段与 label。
  const onModalOk = useCallback(() => {
    if (!modal) return;
    const v = (modal.value || "").trim();
    if (v === "") {
      message.warning("请输入参数");
      return;
    }
    const { mod, param } = modal;
    if (param === "count_n") {
      const n = parseInt(v, 10);
      if (!Number.isFinite(n) || n < 0) {
        message.warning("请输入非负整数");
        return;
      }
      pushBlock(mod, `{${n}}`, `精确 ${n} 次 {${n}}`);
    } else if (param === "count_range") {
      // 形如 "2," 或 "2,4" 或 ",4"
      const parts = v.split(",");
      const mStr = (parts[0] || "").trim();
      const nStr = (parts[1] || "").trim();
      if (mStr === "" && nStr === "") {
        message.warning("请输入 m 或 m,n");
        return;
      }
      const regex = `{${mStr},${nStr}}`;
      pushBlock(mod, regex, `范围 ${v} ${regex}`);
    } else if (param === "literal") {
      const esc = escapeRegexLiteral(v);
      pushBlock(mod, esc, `字面量 ${v}`);
    } else if (param === "charset") {
      // 用户输入字符集内容，前端补方括号。] 需转义。
      let inner = v;
      if (inner.includes("]")) {
        inner = inner.replace(/\]/g, "\\]");
      }
      pushBlock(mod, `[${inner}]`, `字符集 [${v}]`);
    }
    setModal(null);
    setTestError(null);
    setTestHighlight(null);
  }, [modal, pushBlock, message]);

  const onUndo = useCallback(() => {
    setBlocks((prev) => prev.slice(0, -1));
    setTestError(null);
    setTestHighlight(null);
  }, []);

  const onClear = useCallback(() => {
    setBlocks([]);
    setTestError(null);
    setTestHighlight(null);
  }, []);

  const onRemoveBlock = useCallback((key) => {
    setBlocks((prev) => prev.filter((b) => b.key !== key));
    setTestError(null);
    setTestHighlight(null);
  }, []);

  const onLoadPreset = useCallback((preset) => {
    setBlocks(
      preset.blocks.map((b) => ({
        key: `b${blockSeq++}`,
        mid: b.mid,
        label: b.label,
        regex: b.regex,
      }))
    );
    setTestError(null);
    setTestHighlight(null);
  }, []);

  // 测试：纯前端 RegExp 执行 + 高亮。不调用后端。
  const onTest = useCallback(() => {
    if (!pattern) {
      message.warning("请先添加积木构造正则");
      return;
    }
    if (!testInput) {
      message.warning("请输入测试样例");
      return;
    }
    setTestError(null);
    try {
      const re = new RegExp(pattern);
      setTestHighlight(highlightMatches(testInput, re));
    } catch (e) {
      setTestHighlight(null);
      setTestError(`本地 RegExp 执行失败: ${String(e).replace(/^Error: /, "")}`);
    }
  }, [pattern, testInput, message]);

  const hasMatch = useMemo(() => {
    if (!testHighlight) return false;
    return testHighlight.some((n) => n && n.type === "mark");
  }, [testHighlight]);

  return (
    <Card title="正则构造（可视化积木）" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {"点选下方积木按顺序拼装正则，实时预览 + 测试。量词（+ / * / ? / {n}）追加到前一个 token 之后。纯本地拼装，不上传任何数据。"}
        </Text>

        {/* 预设模板 */}
        <div>
          <Text type="secondary" style={{ fontSize: 12, marginRight: 8 }}>
            预设：
          </Text>
          {PRESETS.map((p) => (
            <Button
              key={p.key}
              size="small"
              icon={<PlusOutlined />}
              onClick={() => onLoadPreset(p)}
              style={{ marginRight: 8, marginBottom: 4 }}
            >
              {p.name}
            </Button>
          ))}
        </div>

        {/* 积木模块分组 */}
        {CATEGORY_META.map((cat) => (
          <div key={cat.key}>
            <Text type="secondary" style={{ fontSize: 12, marginRight: 8 }}>
              {cat.title}：
            </Text>
            {BLOCK_MODULES.filter((m) => m.cat === cat.key).map((m) => (
              <Tooltip
                key={m.mid}
                title={m.param ? "点击输入参数" : `追加 ${m.regex || m.label}`}
              >
                <Button
                  size="small"
                  style={{ margin: "2px 4px 2px 0" }}
                  onClick={() => onModuleClick(m)}
                >
                  {m.label}
                </Button>
              </Tooltip>
            ))}
          </div>
        ))}

        {/* 积木序列 + 操作 */}
        <Card
          title="已拼装积木"
          size="small"
          styles={{ body: { padding: 12 } }}
          extra={
            <Space size="small">
              <Button
                size="small"
                icon={<UndoOutlined />}
                onClick={onUndo}
                disabled={blocks.length === 0}
              >
                撤销
              </Button>
              <Button
                size="small"
                danger
                icon={<ClearOutlined />}
                onClick={onClear}
                disabled={blocks.length === 0}
              >
                清空
              </Button>
            </Space>
          }
        >
          {blocks.length > 0 ? (
            <Space size={4} wrap align="center">
              {blocks.map((b, i) => (
                <React.Fragment key={b.key}>
                  <Tag
                    closable
                    onClose={(e) => {
                      e.preventDefault();
                      onRemoveBlock(b.key);
                    }}
                    style={{ margin: 2 }}
                  >
                    {b.label}
                  </Tag>
                  {i < blocks.length - 1 ? (
                    <Text type="secondary" style={{ fontSize: 10 }}>
                      →
                    </Text>
                  ) : null}
                </React.Fragment>
              ))}
            </Space>
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description="点击上方积木或载入预设开始构造"
            />
          )}
        </Card>

        {/* 拼装结果 */}
        {pattern ? (
          <Card
            title="构造结果"
            size="small"
            styles={{ body: { padding: 12 } }}
            extra={
              <Button
                size="small"
                icon={<CopyOutlined />}
                onClick={() => {
                  navigator.clipboard?.writeText(pattern);
                  message.success("已复制");
                }}
              >
                复制
              </Button>
            }
          >
            <Paragraph copyable style={{ fontFamily: "monospace", marginBottom: 12 }}>
              {pattern}
            </Paragraph>

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
              <Alert type="error" showIcon message={testError} style={{ marginTop: 8 }} />
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
            description="添加积木后查看构造结果"
          />
        )}
      </Space>

      {/* 参数收集弹窗 */}
      <Modal
        title={modal ? modal.title : ""}
        open={modal != null}
        onOk={onModalOk}
        onCancel={() => setModal(null)}
        okText="追加"
        cancelText="取消"
        destroyOnClose
      >
        {modal && modal.kind === "number" ? (
          <InputNumber
            autoFocus
            style={{ width: "100%" }}
            placeholder={modal.placeholder}
            value={modal.value}
            min={0}
            onChange={(v) => setModal({ ...modal, value: v != null ? String(v) : "" })}
            onPressEnter={onModalOk}
          />
        ) : (
          <Input
            autoFocus
            placeholder={modal ? modal.placeholder : ""}
            value={modal ? modal.value : ""}
            onChange={(e) => setModal({ ...modal, value: e.target.value })}
            onPressEnter={onModalOk}
          />
        )}
      </Modal>
    </Card>
  );
}

export default ConstructTab;
