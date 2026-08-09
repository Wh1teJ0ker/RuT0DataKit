import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Button,
  Card,
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
  updateRuleParams,
  updateRuleTemplate,
} from "../../tauri";
import {
  DEFAULT_MASK_CHAR,
  EMPTY_TEMPLATE,
  MASK_PRESETS,
  buildTemplateForRun,
  detectPreset,
  normalizeTemplate,
  previewMask,
  templateFromPreset,
} from "./maskTemplate";

const { Title, Text } = Typography;

// v1.1.0 规则管理面板（主区两栏布局）。
// 左：规则列表，按 kind 标签分组（脱敏/校验/提取）；右：选中规则详情。
// 内联测试：三种 kind 全部走前端内部输入 + 正则预览，不写 DB。
//   - validate：new RegExp(pattern).test(input) → 通过/不通过
//   - extract：new RegExp(pattern,'g').matchAll(input) → 命中列表
//   - mask：按掩码字符（默认 *）对输入文本预览脱敏结果
//     · general-mask（T51）：用 6 个模板参数走 apply_template 等价逻辑
//       （keep_prefix/keep_suffix/mask_char/mask_min_len + min_len/max_len guard）
//     · name-mask：旧逻辑（≥3 保留首尾，2 保留首字符）
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
  // T51：general-mask 的 6 个模板参数草稿（camelCase，对齐后端 TemplateParams）。
  const [draftTemplate, setDraftTemplate] = useState({ ...EMPTY_TEMPLATE });
  const [presetKey, setPresetKey] = useState("empty");
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
      // T51：general-mask 初始化 6 个模板参数草稿（从 DB 读取的 template）。
      if (selected.id === "general-mask") {
        const t = normalizeTemplate(selected.template);
        setDraftTemplate(t);
        setPresetKey(detectPreset(t));
      } else {
        setDraftTemplate({ ...EMPTY_TEMPLATE });
        setPresetKey("empty");
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
      // T51：general-mask 持久化 template（6 个模板参数）；
      // 其他 mask 规则持久化 replacement（掩码字符）；validate/extract 持久化 pattern。
      if (selected.id === "general-mask") {
        const tpl = buildTemplateForRun(draftTemplate);
        await updateRuleTemplate(selected.id, tpl);
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
  // T51：general-mask 走模板参数（keep_prefix/keep_suffix/mask_char/mask_min_len +
  // min_len/max_len guard），与后端 apply_template 等价；name-mask 走旧逻辑
  // （≥3 保留首尾，2 保留首字符，1 全掩码，0 空串）。
  const runMaskTest = () => {
    const input = testInput || "";
    if (selected.id === "general-mask") {
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
                      selected.id === "general-mask" ? (
                        // T51：general-mask 暴露 6 个模板参数 + 预设下拉
                        // （与脱敏面板一致，便于在规则管理里直接编辑/测试）。
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
                    ) : (
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
                          if (selected.id === "general-mask") {
                            const t = normalizeTemplate(selected.template);
                            setDraftTemplate(t);
                            setPresetKey(detectPreset(t));
                          } else if (selected.kind === "mask") {
                            setDraftReplacement(
                              selected.replacement &&
                                selected.replacement.length > 0
                                ? selected.replacement
                                : DEFAULT_MASK_CHAR
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
