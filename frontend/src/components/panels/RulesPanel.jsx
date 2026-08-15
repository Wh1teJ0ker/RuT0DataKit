import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Checkbox,
  Col,
  Empty,
  Form,
  Input,
  InputNumber,
  List,
  Row,
  Select,
  Space,
  Spin,
  Switch,
  Tag,
  Typography,
  message,
} from "antd";
import { useAppContext } from "../../state";
import { useBreakpoint } from "../../hooks/useBreakpoint";
import {
  toggleRule,
  updateRuleExtractConfig,
  updateRuleParams,
  updateRuleTemplate,
} from "../../tauri";
// v1.2.0 T94：RulesPanel 需要「全部规则」（不分 kind）用于左侧列表分组，
// useRules(null) 等价于 listRules + setRules，同时获得 reload 方法。
import { useRules } from "../../hooks/useRules";
import TemplateEditor from "../shared/TemplateEditor";
import {
  DEFAULT_MASK_CHAR,
  EMPTY_SEGMENT_TEMPLATE,
  EMPTY_TEMPLATE,
  buildTemplateForRun,
  detectPreset,
  normalizeTemplate,
  previewMask,
} from "./maskTemplate";
// v1.1.4 续轮 T71：校验规则参数共享模块（Generic 参数构造 + 校验提示文案）。
import {
  EMPTY_GENERIC_PARAMS,
  normalizeGenericParams,
  buildGenericParamsForRun,
  VALIDATE_HINTS,
} from "./validateParams";
// v1.2.0：前端内联校验器 — 镜像后端 func_validator.rs 的程序化校验逻辑。
import { runInlineValidator } from "../../utils/inlineValidators";

const { Title, Text } = Typography;

// 内置提取规则的出厂正则（与 crates/core RuleRegistry 保持一致）。
// 重置按钮恢复出厂值时使用，确保"重置"= 回到初始状态而非 DB 当前值。
const BUILTIN_PATTERNS = {
  "phone-extract": "\\b[1-9]\\d{10}\\b",
  "idcard-extract": "\\b[1-9]\\d{16}[\\dXx]\\b",
  "bankcard-extract": "\\b[1-9]\\d{12,18}\\b",
  "ip4-extract": "\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b",
  "ip6-extract": "(?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}",
  "name-extract": "[\\u4e00-\\u9fff]{2,4}",
  "name-validate": "^[\\u4e00-\\u9fa5]{2,4}$",
};

// v1.1.0 规则管理面板（主区两栏布局）。
// 左：规则列表，按 kind 标签分组（脱敏/校验/提取）；右：选中规则详情。
// 内联测试：三种 kind 全部走前端内部输入 + 正则预览，不写 DB。
//   - validate：new RegExp(pattern).test(input) → 通过/不通过
//   - extract：new RegExp(pattern,'g').matchAll(input) → 命中列表
//   - mask：按掩码字符（默认 *）对输入文本预览脱敏结果
//     · simple-mask（T51/T54）：整段脱敏，用 7 个模板参数走 apply_template 等价逻辑
//       （keep_prefix/keep_suffix/mask_char/mask_min_len + min_len/max_len guard + T53 reverse）
//     · segment-mask（T52/T54）：分段脱敏，按分隔符拆分后对指定段脱敏。
//     · name-mask：旧逻辑（≥3 保留首尾，2 保留首字符）
//   - T54：原 general-mask 拆为 simple-mask（整段）+ segment-mask（分段）两条独立规则，
//     各承载一种模板类型，不再需要「模板类型」Select 切换。
// v1.1.4 续轮 T71：validate 规则校验类型只读 Tag 文案映射。
const VALIDATE_LABELS = {
  generic: "通用校验",
  phonePrefix: "手机前缀",
  luhn: "Luhn",
  ipv4: "IPv4",
  ipv6: "IPv6",
  idcard: "身份证",
  username: "用户名",
  sex: "性别",
  birth: "出生日期",
  address: "地址",
};

const KIND_LABEL = { mask: "脱敏", validate: "校验", extract: "提取" };
const KIND_COLOR = { mask: "orange", validate: "red", extract: "blue" };
const KIND_ORDER = ["mask", "validate", "extract"];

export default function RulesPanel() {
  const { state } = useAppContext();
  // v1.2.0 T94：useRules(null) 加载全部规则，reload/loading 供面板复用。
  const { rules, loading, reload } = useRules(null);
  const [selectedId, setSelectedId] = useState(null);
  // 可填参数本地编辑态（保存前不提交）。
  const [draftPattern, setDraftPattern] = useState(null);
  const [draftReplacement, setDraftReplacement] = useState(null);
  // T51：模板脱敏（simple-mask/segment-mask）的模板参数草稿（camelCase，对齐后端 TemplateParams）。
  // T54：原 general-mask 拆为 simple-mask（整段，Simple 模板）+ segment-mask（分段，Segment 模板），
  //   每条规则固定一种模板类型，草稿态用对应空模板初始化。
  const [draftTemplate, setDraftTemplate] = useState({ ...EMPTY_TEMPLATE });
  const [presetKey, setPresetKey] = useState("empty");
  // T55：提取规则编辑态。phonePrefix 的允许前缀草稿（空数组=默认1开头）。
  const [draftAllowedPrefixes, setDraftAllowedPrefixes] = useState([]);
  // idcard-extract：是否允许首位为 0（勾选后 pattern 改为 \b\d{17}[\dXx]\b）。
  const [draftAllowLeadingZero, setDraftAllowLeadingZero] = useState(false);
  // v1.1.4 续轮 T71：generic-validate 参数草稿（字符类 + 长度限制）。
  const [draftGenericParams, setDraftGenericParams] = useState({ ...EMPTY_GENERIC_PARAMS });
  const [saving, setSaving] = useState(false);
  // 测试输入 + 结果。
  const [testInput, setTestInput] = useState("");
  const [testResult, setTestResult] = useState(null);
  const [testing, setTesting] = useState(false);

  // v1.2.0 T94：refresh 收敛为直接调 reload（loading 由 useRules 管理）。
  const refresh = async () => {
    await reload();
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 首次 rules 加载后默认选中第一条。
  useEffect(() => {
    if (rules.length && !selectedId) {
      setSelectedId(rules[0].id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rules]);

  const selected = useMemo(
    () => rules.find((r) => r.id === selectedId) || null,
    [rules, selectedId]
  );

  // 选中规则变化时，同步可填参数草稿 + 清空测试结果。
  useEffect(() => {
    if (selected) {
      setDraftPattern(selected.pattern ?? "");
      // mask 规则：replacement None/空 → 默认 *（草稿态显示默认值）。
      if (selected.kind === "mask") {
        setDraftReplacement(
          selected.replacement && selected.replacement.length > 0
            ? selected.replacement
            : DEFAULT_MASK_CHAR
        );
      } else {
        setDraftReplacement(selected.replacement ?? "");
      }
      // T51/T54：模板脱敏规则（simple-mask/segment-mask）初始化模板参数草稿（从 DB 读取的 template）。
      // simple-mask 走 Simple 模板（EMPTY_TEMPLATE），segment-mask 走 Segment 模板（EMPTY_SEGMENT_TEMPLATE）。
      if (selected.id === "simple-mask") {
        const t = normalizeTemplate(selected.template);
        setDraftTemplate(t);
        setPresetKey(detectPreset(t));
      } else if (selected.id === "segment-mask") {
        const t = normalizeTemplate(selected.template);
        setDraftTemplate(t);
        setPresetKey("custom");
      } else {
        setDraftTemplate({ ...EMPTY_TEMPLATE });
        setPresetKey("empty");
      }
      // T55：提取规则带 params 时初始化允许前缀草稿。
      // phonePrefix 的 allowedPrefixes 空 = 默认（正则保证 1 开头）。
      // v1.1.4 续轮 T71 rework：条件扩展为 extract||validate，与可填参数 Card 分支一致，
      // 否则选中 phone-validate（kind=validate）时草稿被清空，用户直接保存会覆盖 DB 已配置前缀。
      if (
        (selected.kind === "extract" ||
          selected.kind === "validate") &&
        selected.params?.validator === "phonePrefix"
      ) {
        setDraftAllowedPrefixes(
          Array.isArray(selected.params.allowedPrefixes)
            ? selected.params.allowedPrefixes.slice()
            : []
        );
      } else {
        setDraftAllowedPrefixes([]);
      }
      // v1.1.4 续轮 T71：generic-validate 参数草稿初始化。
      // 仅 generic validator 走字符类 + 长度草稿；其他规则走空默认。
      if (selected.params?.validator === "generic") {
        setDraftGenericParams(normalizeGenericParams(selected.params));
      } else {
        setDraftGenericParams({ ...EMPTY_GENERIC_PARAMS });
      }
      // idcard-extract：从 DB pattern 推断"允许首位为 0"初始状态。
      // pattern 含 [1-9] → false（首位非零），否则 → true（宽松召回）。
      // 仅对 idcard-extract 生效（idcard-validate 无 pattern）。
      if (
        selected.params?.validator === "idcard" &&
        selected.kind === "extract"
      ) {
        setDraftAllowLeadingZero(
          selected.pattern && !selected.pattern.includes("[1-9]")
        );
      } else {
        setDraftAllowLeadingZero(false);
      }
      setTestResult(null);
      setTestInput("");
    }
  }, [selectedId, selected?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // 按 kind 分组（保持 KIND_ORDER 顺序）。
  const grouped = useMemo(() => {
    const m = { mask: [], validate: [], extract: [] };
    rules.forEach((r) => {
      if (m[r.kind]) m[r.kind].push(r);
    });
    return KIND_ORDER.map((k) => ({ kind: k, items: m[k] })).filter(
      (g) => g.items.length > 0
    );
  }, [rules]);

  const handleToggle = async (enabled) => {
    if (!selected) return;
    try {
      await toggleRule(selected.id, enabled);
      await refresh();
      message.success(enabled ? "已启用" : "已禁用");
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("toggleRule failed:", e);
      message.error(`切换失败：${e}`);
    }
  };

  const handleSaveParams = async () => {
    if (!selected) return;
    setSaving(true);
    try {
      // T51/T54：模板脱敏规则（simple-mask/segment-mask）持久化 template；
      // 其他 mask 规则持久化 replacement（掩码字符）；
      // T55：extract 规则带 params 时持久化 pattern + params；普通 validate 持久化 pattern。
      if (
        selected.id === "simple-mask" ||
        selected.id === "segment-mask"
      ) {
        const tpl = buildTemplateForRun(draftTemplate);
        await updateRuleTemplate(selected.id, tpl);
      } else if (selected.params?.validator === "generic") {
        // v1.1.4 续轮 T71：generic-validate 走专用的 extract config 命令
        // （写 params，pattern 传 null：generic 不依赖正则）。
        await updateRuleExtractConfig(
          selected.id,
          null,
          buildGenericParamsForRun(draftGenericParams)
        );
      } else if (
        (selected.kind === "extract" || selected.kind === "validate") &&
        selected.params
      ) {
        // T55：提取/校验规则带 params 时持久化 pattern + params。
        // phonePrefix 用允许前缀草稿；其他 validator 原样回写。
        const pattern = draftPattern ?? null;
        let params = selected.params;
        if (params.validator === "phonePrefix") {
          params = {
            validator: "phonePrefix",
            allowedPrefixes: draftAllowedPrefixes.slice(),
          };
        }
        await updateRuleExtractConfig(selected.id, pattern, params);
      } else {
        const pattern = draftPattern ?? null;
        const replacement = draftReplacement ?? null;
        await updateRuleParams(selected.id, pattern, replacement);
      }
      await refresh();
      message.success("参数已保存");
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("save rule params failed:", e);
      message.error(`保存失败：${e}`);
    } finally {
      setSaving(false);
    }
  };

  // v1.2.0 T95：模板编辑 UI 收敛到 TemplateEditor，这里只保留选中规则同步草稿逻辑。
  // handlePresetChange / handleTemplateFieldChange / handleSegmentFieldChange /
  // handleAddSegment / handleRemoveSegment 全部移入 TemplateEditor 内部。

  // 内联测试：根据规则类型走不同路径（全部前端纯逻辑，不写 DB）。
  const handleTest = () => {
    if (!selected) return;
    setTesting(true);
    setTestResult(null);
    try {
      if (selected.kind === "mask") {
        runMaskTest();
      } else if (selected.kind === "validate") {
        runValidateTest();
      } else if (selected.kind === "extract") {
        runExtractTest();
      }
    } finally {
      setTesting(false);
    }
  };

  // validate 测试：优先走程序化校验器（luhn/ipv4/idcard 等），fallback 正则。
  const runValidateTest = () => {
    // 有 params.validator 的程序化校验规则（luhn/ipv4/ipv6/idcard/username/sex/birth/address/generic/phonePrefix）。
    if (selected.params?.validator) {
      const params = selected.params.validator === "phonePrefix"
        ? { ...selected.params, allowedPrefixes: draftAllowedPrefixes }
        : selected.params.validator === "generic"
          ? buildGenericParamsForRun(draftGenericParams)
          : selected.params;
      const result = runInlineValidator(testInput, selected.params.validator, params);
      setTestResult({
        ok: true,
        kind: "validate",
        passed: result.passed,
        message: result.message,
        note: result.note,
      });
      return;
    }
    // 无 params 的 validate 规则（如 name-validate）：走正则。
    const p = draftPattern || selected.pattern;
    if (!p) {
      message.warning("该规则未配置 pattern");
      return;
    }
    let re;
    try {
      re = new RegExp(p);
    } catch (e) {
      setTestResult({ ok: false, error: `正则编译失败：${e.message}` });
      return;
    }
    const passed = re.test(testInput || "");
    setTestResult({
      ok: true,
      kind: "validate",
      passed,
      message: passed ? "通过：输入符合规则" : "不通过：输入不符合规则",
    });
  };

  // extract：前端正则提取预览（不写 DB）。
  const runExtractTest = () => {
    const p = draftPattern || selected.pattern;
    if (!p) {
      message.warning("该规则未配置 pattern");
      return;
    }
    let re;
    try {
      re = new RegExp(p, "g");
    } catch (e) {
      setTestResult({ ok: false, error: `正则编译失败：${e.message}` });
      return;
    }
    const input = testInput || "";
    const hits = [...input.matchAll(re)].map((m) => ({
      value: m[0],
      start: m.index,
      end: m.index + m[0].length,
    }));
    setTestResult({ ok: true, kind: "extract", hits });
  };

  // mask 预览（不写 DB）。
  // T51/T54：simple-mask 走模板参数（keep_prefix/keep_suffix/mask_char/mask_min_len +
  // min_len/max_len guard + T53 reverse），与后端 apply_template 等价；
  // segment-mask 走分段脱敏（按分隔符拆分后对指定段脱敏）；name-mask 走旧逻辑
  // （≥3 保留首尾，2 保留首字符，1 全掩码，0 空串）。
  const runMaskTest = () => {
    const input = testInput || "";
    if (
      selected.id === "simple-mask" ||
      selected.id === "segment-mask"
    ) {
      const fallback = draftTemplate.maskChar || DEFAULT_MASK_CHAR;
      const r = previewMask(draftTemplate, input, fallback);
      setTestResult({
        ok: true,
        kind: "mask",
        output: r.output,
        input,
        maskChar: fallback,
        passthrough: r.passthrough || false,
        skipped: r.skipped || false,
        template: true,
      });
      return;
    }
    // name-mask（无模板）：旧逻辑。
    const draft = draftReplacement ?? "";
    const maskChar = draft.length > 0 ? draft[0] : DEFAULT_MASK_CHAR;
    const chars = [...input];
    const n = chars.length;
    let output;
    if (n === 0) output = "";
    else if (n === 1) output = maskChar;
    else if (n === 2) output = chars[0] + maskChar;
    else output = chars[0] + maskChar.repeat(n - 2) + chars[n - 1];
    setTestResult({ ok: true, kind: "mask", output, input, maskChar });
  };

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const { isCompact } = useBreakpoint();

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <div style={{ padding: "12px 16px 4px", flexShrink: 0 }}>
        <Title level={5} style={{ margin: 0 }}>
          规则管理
        </Title>
      </div>
      <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
        {loading && (
          <div
            style={{
              position: "absolute",
              inset: 0,
              zIndex: 10,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              background: "rgba(255,255,255,0.6)",
            }}
          >
            <Spin />
          </div>
        )}
        <Row gutter={0} style={{ height: "100%" }}>
          {/* 左：规则列表（按 kind 分组） */}
          <Col
            {...(isCompact
              ? { span: 24 }
              : { flex: "240px" })}
            style={{
              borderRight: isCompact ? "none" : "1px solid #f0f0f0",
              borderBottom: isCompact ? "1px solid #f0f0f0" : "none",
              overflow: "auto",
              height: "100%",
              maxHeight: isCompact ? 200 : undefined,
            }}
          >
            <div style={{ padding: 8 }}>
              {grouped.length === 0 ? (
                <Empty
                  description="无规则"
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                />
              ) : (
                grouped.map((g) => (
                  <div key={g.kind} style={{ marginBottom: 8 }}>
                    <div style={{ padding: "4px 8px" }}>
                      <Tag color={KIND_COLOR[g.kind]} style={{ margin: 0 }}>
                        {KIND_LABEL[g.kind]}
                      </Tag>
                    </div>
                    <List
                      size="small"
                      split={false}
                      dataSource={g.items}
                      renderItem={(r) => (
                        <List.Item
                          onClick={() => setSelectedId(r.id)}
                          style={{
                            cursor: "pointer",
                            background:
                              r.id === selectedId ? "#e6f4ff" : "transparent",
                            padding: "6px 12px",
                            borderRadius: 4,
                            border: "none",
                          }}
                        >
                          <div style={{ width: "100%" }}>
                            <div
                              style={{
                                fontWeight: r.id === selectedId ? 600 : 400,
                                fontSize: 13,
                              }}
                            >
                              {r.name}
                            </div>
                            {!r.enabled && (
                              <Tag
                                color="default"
                                style={{ margin: "4px 0 0", fontSize: 11 }}
                              >
                                已禁用
                              </Tag>
                            )}
                          </div>
                        </List.Item>
                      )}
                    />
                  </div>
                ))
              )}
            </div>
          </Col>

          {/* 右：详情 */}
          <Col
            {...(isCompact
              ? { span: 24 }
              : { flex: "auto" })}
            style={{ overflow: "auto", height: "100%" }}
          >
            {selected ? (
              <div style={{ padding: "16px 24px" }}>
                <Title level={4} style={{ marginBottom: 4 }}>
                  {selected.name}
                </Title>

                <Card
                  size="small"
                  title="规则信息"
                  style={{ marginBottom: 16 }}
                >
                  <Row gutter={[8, 8]}>
                    <Col xs={{ span: 24 }} md={{ span: 6 }}>
                      <Text type="secondary">规则类别</Text>
                    </Col>
                    <Col xs={{ span: 24 }} md={{ span: 18 }}>
                      <Tag color={KIND_COLOR[selected.kind]}>
                        {KIND_LABEL[selected.kind] || selected.kind}
                      </Tag>
                    </Col>
                    <Col xs={{ span: 24 }} md={{ span: 6 }}>
                      <Text type="secondary">说明</Text>
                    </Col>
                    <Col xs={{ span: 24 }} md={{ span: 18 }}>
                      <Text>{selected.description || "（无）"}</Text>
                    </Col>
                    <Col xs={{ span: 24 }} md={{ span: 6 }}>
                      <Text type="secondary">启用</Text>
                    </Col>
                    <Col xs={{ span: 24 }} md={{ span: 18 }}>
                      <Switch
                        size="small"
                        checked={selected.enabled}
                        onChange={handleToggle}
                      />
                    </Col>
                  </Row>
                </Card>

                {/* 可填参数 */}
                <Card
                  size="small"
                  title="可填参数"
                  style={{ marginBottom: 16 }}
                >
                  <Form layout="vertical" size="small">
                    {selected.kind === "mask" ? (
                      selected.id === "segment-mask" ? (
                        // v1.2.0 T95：segment-mask 模板编辑收敛到 TemplateEditor。
                        <TemplateEditor
                          mode="segment"
                          template={draftTemplate}
                          setTemplate={setDraftTemplate}
                          presetKey={presetKey}
                          setPresetKey={setPresetKey}
                        />
                      ) : selected.id === "simple-mask" ? (
                        // v1.2.0 T95：simple-mask 模板编辑收敛到 TemplateEditor。
                        <TemplateEditor
                          mode="simple"
                          template={draftTemplate}
                          setTemplate={setDraftTemplate}
                          presetKey={presetKey}
                          setPresetKey={setPresetKey}
                        />
                      ) : (
                        <Form.Item label="掩码字符" extra="默认 *，保留首尾">
                          <Input
                            value={draftReplacement ?? ""}
                            onChange={(e) =>
                              setDraftReplacement(e.target.value)
                            }
                            placeholder={DEFAULT_MASK_CHAR}
                          />
                        </Form.Item>
                      )
                    ) : (selected.kind === "extract" || selected.kind === "validate") && selected.params ? (
                      // T55/T55b/T55c/T71：extract/validate 规则带 params 时按
                      //   params.validator 分支渲染：
                      //   - generic：字符类 Checkbox + 特殊符号白名单 Input + 长度 InputNumber
                      //   - phonePrefix：校验类型 Tag + 允许前缀 Select + 正则输入框
                      //   - 其他 validator（luhn/ipv4/ipv6/idcard/username/sex/birth/address）：
                      //     只读 Tag + hint 文案（不显示空正则输入框）
                      selected.params.validator === "generic" ? (
                        // v1.1.4 续轮 T71：generic-validate 字符类 + 长度限制。
                        // T77：特殊符号从 Checkbox 全开/全关改为自定义白名单 Input。
                        <>
                          <Form.Item label="校验类型">
                            <Tag color="blue" style={{ margin: 0 }}>
                              通用校验
                            </Tag>
                          </Form.Item>
                          <Form.Item label="允许的字符类">
                            <Checkbox.Group
                              value={[
                                draftGenericParams.allowDigits && "digits",
                                draftGenericParams.allowLetters && "letters",
                              ].filter(Boolean)}
                              onChange={(vals) =>
                                setDraftGenericParams((prev) => ({
                                  ...prev,
                                  allowDigits: vals.includes("digits"),
                                  allowLetters: vals.includes("letters"),
                                }))
                              }
                              options={[
                                { label: "纯数字 (0-9)", value: "digits" },
                                { label: "纯字母 (a-zA-Z)", value: "letters" },
                              ]}
                            />
                          </Form.Item>
                          <Form.Item label="允许的特殊符号">
                            <Input
                              value={draftGenericParams.allowSpecialChars}
                              onChange={(e) =>
                                setDraftGenericParams((prev) => ({
                                  ...prev,
                                  allowSpecialChars: e.target.value,
                                }))
                              }
                              placeholder="留空=不允许；如 _-.@"
                              allowClear
                            />
                          </Form.Item>
                          <Form.Item label="最小长度（空=不限）">
                            <InputNumber
                              value={draftGenericParams.minLen}
                              onChange={(v) =>
                                setDraftGenericParams((prev) => ({
                                  ...prev,
                                  minLen: v,
                                }))
                              }
                              placeholder="不限"
                              min={0}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                          <Form.Item label="最大长度（空=不限）">
                            <InputNumber
                              value={draftGenericParams.maxLen}
                              onChange={(v) =>
                                setDraftGenericParams((prev) => ({
                                  ...prev,
                                  maxLen: v,
                                }))
                              }
                              placeholder="不限"
                              min={0}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                          <Text type="secondary">
                            {VALIDATE_HINTS["generic-validate"] || ""}
                          </Text>
                        </>
                      ) : selected.params.validator === "phonePrefix" ? (
                        // phonePrefix（extract/validate 共用）：Tag + 允许前缀 + 正则输入框。
                        <>
                          <Form.Item label="校验类型">
                            <Tag color="blue" style={{ margin: 0 }}>
                              手机前缀
                            </Tag>
                          </Form.Item>
                          <Form.Item
                            label="允许前缀"
                            extra="3 位前缀回车添加，留空=默认 1 开头"
                          >
                            <Select
                              mode="tags"
                              value={draftAllowedPrefixes}
                              onChange={(v) => setDraftAllowedPrefixes(v)}
                              placeholder="回车添加前缀"
                              tokenSeparators={[",", " ", "\n"]}
                              open={false}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                          <Form.Item label="正则模式">
                            <Input
                              value={draftPattern ?? ""}
                              onChange={(e) => setDraftPattern(e.target.value)}
                              placeholder="提取正则"
                            />
                          </Form.Item>
                        </>
                      ) : selected.params.validator === "idcard" &&
                        selected.kind === "extract" ? (
                        // idcard-extract：只读 Tag + "允许首位为 0" 开关 + 正则输入框。
                        // 仅对提取规则生效（idcard-validate 无 pattern，不显示）。
                        <>
                          <Form.Item label="校验类型">
                            <Tag color="blue" style={{ margin: 0 }}>
                              身份证
                            </Tag>
                          </Form.Item>
                          <Form.Item label="允许首位为 0">
                            <Switch
                              checked={draftAllowLeadingZero}
                              onChange={(checked) => {
                                setDraftAllowLeadingZero(checked);
                                setDraftPattern(
                                  checked
                                    ? "\\b\\d{17}[\\dXx]\\b"
                                    : "\\b[1-9]\\d{16}[\\dXx]\\b"
                                );
                              }}
                            />
                            <Text
                              type="secondary"
                              style={{ marginLeft: 8, fontSize: 12 }}
                            >
                              {draftAllowLeadingZero
                                ? "首位可为 0（宽松召回）"
                                : "首位必须非零（默认）"}
                            </Text>
                          </Form.Item>
                          <Form.Item label="正则模式">
                            <Input
                              value={draftPattern ?? ""}
                              onChange={(e) => setDraftPattern(e.target.value)}
                              placeholder="提取正则"
                            />
                          </Form.Item>
                        </>
                      ) : (
                        // 其他 validator（luhn/ipv4/ipv6/username/sex/birth/address）：
                        // 只读 Tag + hint 文案，不显示空正则输入框。
                        <>
                          <Form.Item label="校验类型">
                            <Tag color="blue" style={{ margin: 0 }}>
                              {VALIDATE_LABELS[selected.params.validator] ||
                                selected.params.validator}
                            </Tag>
                          </Form.Item>
                          <Text type="secondary">
                            {VALIDATE_HINTS[selected.id] || ""}
                          </Text>
                        </>
                      )
                    ) : (
                      // 无 params 的 validate 规则（如 name-validate）：hint 文案 + 正则输入框。
                      <>
                        {selected.kind === "validate" &&
                          VALIDATE_HINTS[selected.id] && (
                            <Text type="secondary" style={{ display: "block", marginBottom: 8 }}>
                              {VALIDATE_HINTS[selected.id]}
                            </Text>
                          )}
                        <Form.Item label="正则模式">
                          <Input
                            value={draftPattern ?? ""}
                            onChange={(e) => setDraftPattern(e.target.value)}
                            placeholder={
                              selected.kind === "validate"
                                ? "如 ^[\u4e00-\u9fa5]{2,4}$"
                                : "如 [\u4e00-\u9fa5]{2,4}"
                            }
                          />
                        </Form.Item>
                      </>
                    )}
                    <Space>
                      <Button
                        type="primary"
                        loading={saving}
                        onClick={handleSaveParams}
                      >
                        保存参数
                      </Button>
                      <Button
                        onClick={() => {
                          if (selected.id === "simple-mask") {
                            const t = normalizeTemplate(selected.template);
                            setDraftTemplate(t);
                            setPresetKey(detectPreset(t));
                          } else if (selected.id === "segment-mask") {
                            const t = normalizeTemplate(selected.template);
                            setDraftTemplate(t);
                            setPresetKey("custom");
                          } else if (selected.kind === "mask") {
                            setDraftReplacement(
                              selected.replacement &&
                                selected.replacement.length > 0
                                ? selected.replacement
                                : DEFAULT_MASK_CHAR
                            );
                          } else if (
                            selected.params?.validator === "phonePrefix"
                          ) {
                            // 重置 = 恢复出厂正则 + 空前缀列表（默认 1 开头）。
                            setDraftPattern(BUILTIN_PATTERNS[selected.id] ?? "");
                            setDraftAllowedPrefixes([]);
                          } else if (
                            selected.params?.validator === "generic"
                          ) {
                            // v1.1.4 续轮 T71：generic-validate 重置 → 用 DB params 回填。
                            setDraftGenericParams(
                              normalizeGenericParams(selected.params)
                            );
                          } else {
                            // 提取规则（非 phonePrefix）重置为出厂正则；
                            // 其他 validate 规则恢复 DB pattern。
                            setDraftPattern(
                              BUILTIN_PATTERNS[selected.id] ?? selected.pattern ?? ""
                            );
                            // idcard-extract 重置 → 恢复首位非零出厂状态。
                            setDraftAllowLeadingZero(false);
                          }
                        }}
                      >
                        重置
                      </Button>
                    </Space>
                  </Form>
                </Card>

                {/* 内联测试（前端纯逻辑，不写 DB） */}
                <Card size="small" title="内联测试">
                  <Form layout="vertical" size="small">
                    <Form.Item label="测试输入">
                      <Input.TextArea
                        rows={3}
                        value={testInput}
                        onChange={(e) => setTestInput(e.target.value)}
                        placeholder="输入测试文本"
                      />
                    </Form.Item>
                  </Form>
                  <Space style={{ marginTop: 4 }}>
                    <Button loading={testing} onClick={handleTest}>
                      执行测试
                    </Button>
                    <Button
                      onClick={() => {
                        setTestInput("");
                        setTestResult(null);
                      }}
                    >
                      清空
                    </Button>
                  </Space>

                  {testResult && (
                    <div
                      style={{
                        marginTop: 12,
                        maxHeight: "30vh",
                        overflow: "auto",
                      }}
                    >
                      {testResult.error ? (
                        <Alert
                          type="error"
                          showIcon
                          message={testResult.error}
                        />
                      ) : testResult.kind === "validate" ? (
                        <Alert
                          type={testResult.passed ? "success" : "warning"}
                          showIcon
                          message={testResult.message}
                          description={testResult.note ? `附加信息：${testResult.note}` : undefined}
                        />
                      ) : testResult.kind === "extract" ? (
                        testResult.hits.length ? (
                          <List
                            size="small"
                            dataSource={testResult.hits}
                            style={{ maxHeight: "20vh", overflow: "auto" }}
                            renderItem={(h, i) => (
                              <List.Item key={i}>
                                <Text code>{h.value}</Text>
                                <Text type="secondary" style={{ fontSize: 11 }}>
                                  {" "}（{h.start}-{h.end}）
                                </Text>
                              </List.Item>
                            )}
                          />
                        ) : (
                          <Alert type="info" showIcon message="无命中" />
                        )
                      ) : testResult.kind === "mask" ? (
                        <div>
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            原文：
                          </Text>
                          <Text code>{testResult.input || "（空）"}</Text>
                          <br />
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            掩码字符：
                          </Text>
                          <Text code>{testResult.maskChar || DEFAULT_MASK_CHAR}</Text>
                          <br />
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            脱敏后：
                          </Text>
                          <Text code>{testResult.output}</Text>
                          {testResult.template && testResult.passthrough && (
                            <>
                              <br />
                              <Alert
                                type="info"
                                showIcon
                                style={{ marginTop: 8 }}
                                message="空模板，原样返回"
                              />
                            </>
                          )}
                        </div>
                      ) : null}
                    </div>
                  )}
                </Card>
              </div>
            ) : (
              <Empty
                style={{ marginTop: 48 }}
                description="请从左侧选择一条规则"
              />
            )}
          </Col>
        </Row>
      </div>
    </div>
  );
}
