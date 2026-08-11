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
import {
  listRules,
  toggleRule,
  updateRuleExtractConfig,
  updateRuleParams,
  updateRuleTemplate,
} from "../../tauri";
import {
  DEFAULT_MASK_CHAR,
  EMPTY_SEGMENT_TEMPLATE,
  EMPTY_TEMPLATE,
  MASK_PRESETS,
  buildTemplateForRun,
  detectPreset,
  isSegmentTemplate,
  normalizeTemplate,
  previewMask,
  templateFromPreset,
} from "./maskTemplate";
// v1.1.4 续轮 T71：校验规则参数共享模块（Generic 参数构造 + 校验提示文案）。
import {
  EMPTY_GENERIC_PARAMS,
  normalizeGenericParams,
  buildGenericParamsForRun,
  VALIDATE_HINTS,
} from "./validateParams";

const { Title, Text } = Typography;

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
  const [rules, setRules] = useState([]);
  const [loading, setLoading] = useState(false);
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
  // v1.1.4 续轮 T71：generic-validate 参数草稿（字符类 + 长度限制）。
  const [draftGenericParams, setDraftGenericParams] = useState({ ...EMPTY_GENERIC_PARAMS });
  const [saving, setSaving] = useState(false);
  // 测试输入 + 结果。
  const [testInput, setTestInput] = useState("");
  const [testResult, setTestResult] = useState(null);
  const [testing, setTesting] = useState(false);

  const refresh = async () => {
    setLoading(true);
    try {
      const all = await listRules();
      setRules(all);
      if (all.length && !selectedId) {
        setSelectedId(all[0].id);
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("listRules failed:", e);
      message.error(`加载规则失败：${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  // T51：选预设 → 填充 6 参数框（custom 不填充，保留当前用户输入）。
  const handlePresetChange = (key) => {
    setPresetKey(key);
    const filled = templateFromPreset(key);
    if (filled === null) return; // custom → 不填充
    setDraftTemplate(filled);
  };

  // T51：编辑单个模板参数框 → 更新 draftTemplate + 重新检测匹配的预设。
  const handleTemplateFieldChange = (field, value) => {
    setDraftTemplate((prev) => {
      const next = { ...prev, [field]: value };
      setPresetKey(detectPreset(next));
      return next;
    });
  };

  // T52：编辑 Segment 段配置单字段（index/keepPrefix/keepSuffix/maskMinLen）。
  const handleSegmentFieldChange = (segIdx, field, value) => {
    setDraftTemplate((prev) => {
      if (!isSegmentTemplate(prev)) return prev;
      const segments = prev.segments.map((s, i) =>
        i === segIdx ? { ...s, [field]: value } : s,
      );
      return { ...prev, segments };
    });
  };

  // T52：新增段配置（默认 index 自增、keepPrefix=1、keepSuffix=1、maskMinLen=1）。
  const handleAddSegment = () => {
    setDraftTemplate((prev) => {
      if (!isSegmentTemplate(prev)) return prev;
      const nextIdx =
        prev.segments.length > 0
          ? Math.max(...prev.segments.map((s) => Number(s.index) || 0)) + 1
          : 0;
      return {
        ...prev,
        segments: [
          ...prev.segments,
          { index: nextIdx, keepPrefix: 1, keepSuffix: 1, maskMinLen: 1 },
        ],
      };
    });
  };

  // T52：删除段配置。
  const handleRemoveSegment = (segIdx) => {
    setDraftTemplate((prev) => {
      if (!isSegmentTemplate(prev)) return prev;
      return {
        ...prev,
        segments: prev.segments.filter((_, i) => i !== segIdx),
      };
    });
  };

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

  // validate：前端正则预览（不写 DB）。
  const runValidateTest = () => {
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

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <div style={{ padding: "12px 16px 4px", flexShrink: 0 }}>
        <Title level={5} style={{ margin: 0 }}>
          规则管理
        </Title>
      </div>
      <Spin spinning={loading}>
        <Row gutter={0} style={{ height: "calc(100vh - 96px)" }}>
          {/* 左：规则列表（按 kind 分组） */}
          <Col
            flex="240px"
            style={{
              borderRight: "1px solid #f0f0f0",
              overflow: "auto",
              height: "100%",
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
            flex="auto"
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
                    <Col span={6}>
                      <Text type="secondary">规则类别</Text>
                    </Col>
                    <Col span={18}>
                      <Tag color={KIND_COLOR[selected.kind]}>
                        {KIND_LABEL[selected.kind] || selected.kind}
                      </Tag>
                    </Col>
                    <Col span={6}>
                      <Text type="secondary">说明</Text>
                    </Col>
                    <Col span={18}>
                      <Text>{selected.description || "（无）"}</Text>
                    </Col>
                    <Col span={6}>
                      <Text type="secondary">启用</Text>
                    </Col>
                    <Col span={18}>
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
                        // T52/T54：segment-mask 分段脱敏配置（按分隔符拆分后对指定段脱敏）。
                        // 与脱敏面板一致，便于在规则管理里直接编辑/测试。
                        <>
                          <Form.Item label="分隔符" extra="如 @ . - / 等单字符或多字符">
                            <Input
                              value={draftTemplate.delimiter}
                              onChange={(e) =>
                                setDraftTemplate((prev) => ({
                                  ...prev,
                                  delimiter: e.target.value,
                                }))
                              }
                              placeholder="@ / . 等"
                              allowClear
                            />
                          </Form.Item>
                          <Form.Item label="掩码字符" extra="默认 *；取首个字符">
                            <Input
                              value={draftTemplate.maskChar ?? ""}
                              onChange={(e) =>
                                setDraftTemplate((prev) => ({
                                  ...prev,
                                  maskChar: e.target.value || null,
                                }))
                              }
                              placeholder={DEFAULT_MASK_CHAR}
                              allowClear
                            />
                          </Form.Item>
                          <Form.Item label="段配置">
                            <Space direction="vertical" style={{ width: "100%" }}>
                              {(draftTemplate.segments || []).map((seg, i) => (
                                <Card
                                  key={i}
                                  size="small"
                                  title={`段 #${i}`}
                                  headStyle={{ minHeight: 32, padding: "0 8px" }}
                                  bodyStyle={{ padding: 8 }}
                                  extra={
                                    <Button
                                      size="small"
                                      type="text"
                                      onClick={() => handleRemoveSegment(i)}
                                    >
                                      删除
                                    </Button>
                                  }
                                >
                                  <Row gutter={8}>
                                    <Col span={6}>
                                      <Form.Item label="段索引" style={{ marginBottom: 8 }}>
                                        <InputNumber
                                          value={seg.index}
                                          onChange={(v) =>
                                            handleSegmentFieldChange(i, "index", v)
                                          }
                                          min={0}
                                          style={{ width: "100%" }}
                                          size="small"
                                        />
                                      </Form.Item>
                                    </Col>
                                    <Col span={6}>
                                      <Form.Item label="保留前" style={{ marginBottom: 8 }}>
                                        <InputNumber
                                          value={seg.keepPrefix}
                                          onChange={(v) =>
                                            handleSegmentFieldChange(i, "keepPrefix", v)
                                          }
                                          min={0}
                                          style={{ width: "100%" }}
                                          size="small"
                                        />
                                      </Form.Item>
                                    </Col>
                                    <Col span={6}>
                                      <Form.Item label="保留后" style={{ marginBottom: 8 }}>
                                        <InputNumber
                                          value={seg.keepSuffix}
                                          onChange={(v) =>
                                            handleSegmentFieldChange(i, "keepSuffix", v)
                                          }
                                          min={0}
                                          style={{ width: "100%" }}
                                          size="small"
                                        />
                                      </Form.Item>
                                    </Col>
                                    <Col span={6}>
                                      <Form.Item label="最少掩码" style={{ marginBottom: 8 }}>
                                        <InputNumber
                                          value={seg.maskMinLen}
                                          onChange={(v) =>
                                            handleSegmentFieldChange(i, "maskMinLen", v)
                                          }
                                          min={0}
                                          style={{ width: "100%" }}
                                          size="small"
                                        />
                                      </Form.Item>
                                    </Col>
                                  </Row>
                                </Card>
                              ))}
                              <Button size="small" onClick={handleAddSegment}>
                                添加段配置
                              </Button>
                            </Space>
                          </Form.Item>
                        </>
                      ) : selected.id === "simple-mask" ? (
                        // T51/T54：simple-mask 整段脱敏暴露预设下拉 + 7 个模板参数
                        // （keepPrefix/keepSuffix/maskChar/maskMinLen + minLen/maxLen + T53 reverse）。
                        <>
                          <Form.Item
                            label="子规则（预设）"
                            extra="选预设填充参数，可继续修改；空模板=不脱敏（透传）"
                          >
                            <Select
                              value={presetKey}
                              onChange={handlePresetChange}
                              options={MASK_PRESETS.map((p) => ({
                                label: p.label,
                                value: p.key,
                              }))}
                            />
                          </Form.Item>
                          <Form.Item label="保留前缀字符数">
                            <InputNumber
                              value={draftTemplate.keepPrefix}
                              onChange={(v) =>
                                handleTemplateFieldChange("keepPrefix", v)
                              }
                              placeholder="0"
                              min={0}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                          <Form.Item label="保留后缀字符数">
                            <InputNumber
                              value={draftTemplate.keepSuffix}
                              onChange={(v) =>
                                handleTemplateFieldChange("keepSuffix", v)
                              }
                              placeholder="0"
                              min={0}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                          <Form.Item label="掩码字符" extra="默认 *；取首个字符">
                            <Input
                              value={draftTemplate.maskChar ?? ""}
                              onChange={(e) =>
                                handleTemplateFieldChange(
                                  "maskChar",
                                  e.target.value || null
                                )
                              }
                              placeholder={DEFAULT_MASK_CHAR}
                              allowClear
                            />
                          </Form.Item>
                          <Form.Item label="最少掩码字符数">
                            <InputNumber
                              value={draftTemplate.maskMinLen}
                              onChange={(v) =>
                                handleTemplateFieldChange("maskMinLen", v)
                              }
                              placeholder="1"
                              min={0}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                          <Form.Item
                            label="反向脱敏"
                            extra="开启后保留中间，对首 N 位和后 N 位脱敏（上方前后缀位数变为首尾脱码位数）"
                          >
                            <Switch
                              size="small"
                              checked={draftTemplate.reverse === true}
                              onChange={(checked) =>
                                handleTemplateFieldChange(
                                  "reverse",
                                  checked || null
                                )
                              }
                            />
                          </Form.Item>
                          <Form.Item label="值长度下限（guard，空=不限）">
                            <InputNumber
                              value={draftTemplate.minLen}
                              onChange={(v) =>
                                handleTemplateFieldChange("minLen", v)
                              }
                              placeholder="不限"
                              min={0}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                          <Form.Item label="值长度上限（guard，空=不限）">
                            <InputNumber
                              value={draftTemplate.maxLen}
                              onChange={(v) =>
                                handleTemplateFieldChange("maxLen", v)
                              }
                              placeholder="不限"
                              min={0}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                        </>
                      ) : (
                        <Form.Item label="掩码字符" extra="默认 *；取首个字符；保留首尾">
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
                      //   - generic：字符类 Checkbox + 长度 InputNumber
                      //   - phonePrefix：校验类型 Tag + 允许前缀 Select + 正则输入框
                      //   - 其他 validator（luhn/ipv4/ipv6/idcard/username/sex/birth/address）：
                      //     只读 Tag + hint 文案（不显示空正则输入框）
                      selected.params.validator === "generic" ? (
                        // v1.1.4 续轮 T71：generic-validate 字符类 + 长度限制。
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
                                draftGenericParams.allowSpecial && "special",
                              ].filter(Boolean)}
                              onChange={(vals) =>
                                setDraftGenericParams((prev) => ({
                                  ...prev,
                                  allowDigits: vals.includes("digits"),
                                  allowLetters: vals.includes("letters"),
                                  allowSpecial: vals.includes("special"),
                                }))
                              }
                              options={[
                                { label: "纯数字 (0-9)", value: "digits" },
                                { label: "纯字母 (a-zA-Z)", value: "letters" },
                                {
                                  label: "特殊符号（含标点/空格/中文等）",
                                  value: "special",
                                },
                              ]}
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
                            extra="输入 3 位前缀回车添加；空列表=默认 1 开头"
                          >
                            <Select
                              mode="tags"
                              value={draftAllowedPrefixes}
                              onChange={(v) => setDraftAllowedPrefixes(v)}
                              placeholder="留空=默认 1 开头"
                              tokenSeparators={[",", " ", "\n"]}
                              open={false}
                              style={{ width: "100%" }}
                            />
                          </Form.Item>
                          <Form.Item label="正则模式">
                            <Input
                              value={draftPattern ?? ""}
                              onChange={(e) => setDraftPattern(e.target.value)}
                              placeholder="提取正则（宽松召回，严格校验交给校验函数）"
                            />
                          </Form.Item>
                        </>
                      ) : (
                        // 其他 validator（luhn/ipv4/ipv6/idcard/username/sex/birth/address）：
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
                            // v1.1.4 续轮 T71：phonePrefix 重置（extract/validate 共用）。
                            setDraftPattern(selected.pattern ?? "");
                            setDraftAllowedPrefixes(
                              Array.isArray(selected.params.allowedPrefixes)
                                ? selected.params.allowedPrefixes.slice()
                                : []
                            );
                          } else if (
                            selected.params?.validator === "generic"
                          ) {
                            // v1.1.4 续轮 T71：generic-validate 重置 → 用 DB params 回填。
                            setDraftGenericParams(
                              normalizeGenericParams(selected.params)
                            );
                          } else {
                            setDraftPattern(selected.pattern ?? "");
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
                    <div style={{ marginTop: 12 }}>
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
                        />
                      ) : testResult.kind === "extract" ? (
                        testResult.hits.length ? (
                          <List
                            size="small"
                            dataSource={testResult.hits}
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
                                message="空模板（透传）：原样返回，未脱敏"
                              />
                            </>
                          )}
                          {testResult.template && testResult.skipped && (
                            <>
                              <br />
                              <Alert
                                type="warning"
                                showIcon
                                style={{ marginTop: 8 }}
                                message="长度不在 [下限, 上限] 区间内（guard 命中）：原样返回"
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
      </Spin>
    </div>
  );
}
