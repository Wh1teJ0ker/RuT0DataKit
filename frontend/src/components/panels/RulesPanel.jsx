import { useEffect, useMemo, useState } from "react";
import { Col, Empty, List, Row, Spin, Tag, Typography, message } from "antd";
import { useAppContext } from "../../state";
import { useBreakpoint } from "../../hooks/useBreakpoint";
import {
  toggleRule,
  updateRuleExtractConfig,
  updateRuleParams,
  updateRuleTemplate,
} from "../../tauri";
import { useRules } from "../../hooks/useRules";
import {
  DEFAULT_MASK_CHAR,
  EMPTY_TEMPLATE,
  buildTemplateForRun,
  detectPreset,
  normalizeTemplate,
  previewMask,
} from "./maskTemplate";
import {
  EMPTY_GENERIC_PARAMS,
  normalizeGenericParams,
  buildGenericParamsForRun,
} from "./validateParams";
import { runInlineValidator } from "../../utils/inlineValidators";
import { KIND_COLOR, KIND_LABEL, KIND_ORDER } from "./rule/constants";
import RuleDetail from "./rule/RuleDetail";
import RuleTest from "./rule/RuleTest";

const { Title } = Typography;

// v1.1.0 规则管理面板（主区两栏布局）。
// v1.2.1 T9：拆分为 rule/ 子目录（RuleDetail + RuleTest + params/ + constants.js），
//   本文件瘦身为容器：列表 + 选中状态 + 草稿同步 + 保存/重置/测试逻辑。
export default function RulesPanel() {
  const { state } = useAppContext();
  const { rules, loading, reload } = useRules(null);
  const [selectedId, setSelectedId] = useState(null);
  const [draftPattern, setDraftPattern] = useState(null);
  const [draftReplacement, setDraftReplacement] = useState(null);
  const [draftTemplate, setDraftTemplate] = useState({ ...EMPTY_TEMPLATE });
  const [presetKey, setPresetKey] = useState("empty");
  const [draftAllowedPrefixes, setDraftAllowedPrefixes] = useState([]);
  const [draftAllowLeadingZero, setDraftAllowLeadingZero] = useState(false);
  const [draftGenericParams, setDraftGenericParams] = useState({ ...EMPTY_GENERIC_PARAMS });
  const [saving, setSaving] = useState(false);
  const [testInput, setTestInput] = useState("");
  const [testResult, setTestResult] = useState(null);
  const [testing, setTesting] = useState(false);

  const refresh = async () => {
    await reload();
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  // 选中规则变化时，同步草稿 + 清空测试。
  useEffect(() => {
    if (selected) {
      setDraftPattern(selected.pattern ?? "");
      if (selected.kind === "mask") {
        setDraftReplacement(
          selected.replacement && selected.replacement.length > 0
            ? selected.replacement
            : DEFAULT_MASK_CHAR
        );
      } else {
        setDraftReplacement(selected.replacement ?? "");
      }
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
      if (
        (selected.kind === "extract" || selected.kind === "validate") &&
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
      if (selected.params?.validator === "generic") {
        setDraftGenericParams(normalizeGenericParams(selected.params));
      } else {
        setDraftGenericParams({ ...EMPTY_GENERIC_PARAMS });
      }
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

  // 按 kind 分组。
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
      console.error("toggleRule failed:", e);
      message.error(`切换失败：${e}`);
    }
  };

  const handleSaveParams = async () => {
    if (!selected) return;
    setSaving(true);
    try {
      if (selected.id === "simple-mask" || selected.id === "segment-mask") {
        const tpl = buildTemplateForRun(draftTemplate);
        await updateRuleTemplate(selected.id, tpl);
      } else if (selected.params?.validator === "generic") {
        await updateRuleExtractConfig(
          selected.id,
          null,
          buildGenericParamsForRun(draftGenericParams)
        );
      } else if (
        (selected.kind === "extract" || selected.kind === "validate") &&
        selected.params
      ) {
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
      console.error("save rule params failed:", e);
      message.error(`保存失败：${e}`);
    } finally {
      setSaving(false);
    }
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

  const runValidateTest = () => {
    if (selected.params?.validator) {
      const params =
        selected.params.validator === "phonePrefix"
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

  const runMaskTest = () => {
    const input = testInput || "";
    if (selected.id === "simple-mask" || selected.id === "segment-mask") {
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
          {/* 左：规则列表 */}
          <Col
            {...(isCompact ? { span: 24 } : { flex: "240px" })}
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
            {...(isCompact ? { span: 24 } : { flex: "auto" })}
            style={{ overflow: "auto", height: "100%" }}
          >
            {selected ? (
              <>
                <RuleDetail
                  selected={selected}
                  draftPattern={draftPattern}
                  setDraftPattern={setDraftPattern}
                  draftReplacement={draftReplacement}
                  setDraftReplacement={setDraftReplacement}
                  draftTemplate={draftTemplate}
                  setDraftTemplate={setDraftTemplate}
                  presetKey={presetKey}
                  setPresetKey={setPresetKey}
                  draftAllowedPrefixes={draftAllowedPrefixes}
                  setDraftAllowedPrefixes={setDraftAllowedPrefixes}
                  draftAllowLeadingZero={draftAllowLeadingZero}
                  setDraftAllowLeadingZero={setDraftAllowLeadingZero}
                  draftGenericParams={draftGenericParams}
                  setDraftGenericParams={setDraftGenericParams}
                  saving={saving}
                  onSave={handleSaveParams}
                  onToggle={handleToggle}
                />
                <div style={{ padding: "0 24px 16px" }}>
                  <div
                    className="ant-card ant-card-bordered ant-card-small"
                    style={{ marginBottom: 0 }}
                  >
                    <div className="ant-card-head">
                      <div className="ant-card-head-title">内联测试</div>
                    </div>
                    <div className="ant-card-body">
                      <RuleTest
                        testInput={testInput}
                        setTestInput={setTestInput}
                        testResult={testResult}
                        testing={testing}
                        onTest={handleTest}
                        defaultMaskChar={DEFAULT_MASK_CHAR}
                      />
                    </div>
                  </div>
                </div>
              </>
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
