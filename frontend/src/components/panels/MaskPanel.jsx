import { useEffect, useState } from "react";
import { Button, Card, Checkbox, Col, Empty, Form, Input, InputNumber, Row, Select, Space, Switch, message } from "antd";
import { useAppContext } from "../../state";
import {
  maskColumn,
  getSheetData,
  listRules,
  updateRuleParams,
  updateRuleTemplate,
} from "../../tauri";
import { PAGE_SIZE } from "../../constants";
import {
  DEFAULT_MASK_CHAR,
  EMPTY_SEGMENT_TEMPLATE,
  EMPTY_TEMPLATE,
  MASK_PRESETS,
  buildTemplateForRun,
  detectPreset,
  isSegmentTemplate,
  normalizeTemplate,
  resolveMaskChar,
  templateFromPreset,
} from "./maskTemplate";

// v1.1.3 T49 脱敏面板（T54 拆分后）：
//   - 脱敏规则下拉：姓名脱敏（name-mask）+ 整段脱敏（simple-mask）+ 分段脱敏
//     （segment-mask）。4 条原独立规则（身份证/手机/出生日期/银行卡）收敛为
//     simple-mask 的**预设**，不再单独列出。
//   - 选 simple-mask → 显示「子规则（预设）」下拉 + 7 个可编辑参数框（含 T53
//     反向脱敏开关）。选预设即填充参数，用户可继续修改；空模板=不脱敏（透传）。
//   - 选 segment-mask → 显示分隔符 + 段配置（按分隔符拆分后对指定段脱敏）。
//   - 选 name-mask → 仅显示掩码字符输入（旧逻辑，保留首尾各 1）。
//   - 执行脱敏：simple-mask/segment-mask 把当前参数组装成 template 透传给
//     mask_column（临时覆盖，不写回 DB）；name-mask 走旧逻辑。
//   - 保存设置：simple-mask/segment-mask → updateRuleTemplate 持久化 template；
//     name-mask → updateRuleParams 持久化 replacement。
//   - T51：预设表 / 模板组装 / 透传判定抽到 ./maskTemplate.js，与 RulesPanel 共享。
//   - T54：原 general-mask 一条规则拆为 simple-mask + segment-mask 两条独立规则，
//     每条规则固定一种模板类型，移除「模板类型」Select 切换。

export default function MaskPanel() {
  const { state, dispatch, applyRowStatuses } = useAppContext();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [maskRuleId, setMaskRuleId] = useState(null);
  const [maskRules, setMaskRules] = useState([]);
  // simple-mask 的 7 个模板参数（camelCase，对齐后端 TemplateParams::Simple）
  // segment-mask 的分段参数（delimiter + segments[]，对齐 Segment 变体）
  const [presetKey, setPresetKey] = useState("empty");
  const [template, setTemplate] = useState({ ...EMPTY_TEMPLATE });
  // name-mask 的掩码字符（replacement）
  const [nameMaskChar, setNameMaskChar] = useState(DEFAULT_MASK_CHAR);
  // v1.1.5 T85：先校验再脱敏。勾选后先按 validate 规则过滤该列，通过的行脱敏，
  // 未通过行写为 invalidText 占位（默认 "INVALID"）。
  const [validateRules, setValidateRules] = useState([]);
  const [validateEnabled, setValidateEnabled] = useState(false);
  const [validateRuleId, setValidateRuleId] = useState(null);
  const [invalidText, setInvalidText] = useState("INVALID");
  // phone-validate 行级前缀白名单（与 ValidatePanel 模式一致，tags 模式输入）。
  const [phonePrefixesInput, setPhonePrefixesInput] = useState([]);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];
  const isSimpleMask = maskRuleId === "simple-mask";
  const isSegmentMask = maskRuleId === "segment-mask";
  const isTemplateMask = isSimpleMask || isSegmentMask;

  // 加载全部 mask 规则供下拉选择；默认选中首条 mask 规则。
  // T85：同时加载 validate-kind 规则，供「先校验再脱敏」勾选时下拉。
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const all = await listRules();
        if (cancelled) return;
        const masks = all.filter((r) => r.kind === "mask");
        setMaskRules(masks);
        const validates = all.filter((r) => r.kind === "validate");
        setValidateRules(validates);
        if (validates.length > 0) {
          setValidateRuleId(validates[0].id);
        }
        if (masks.length === 0) return;
        const first = masks[0];
        setMaskRuleId(first.id);
        form.setFieldValue("maskRuleId", first.id);
        if ((first.id === "simple-mask" || first.id === "segment-mask") && first.template) {
          initTemplateFromRule(first.template);
        } else if (first.id === "name-mask") {
          const ch = resolveMaskChar(first);
          setNameMaskChar(ch);
          form.setFieldValue("nameMaskChar", ch);
        }
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("load mask rules failed:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 从规则 template 初始化参数框（simple-mask/segment-mask 从 DB 读取的 template）。
  const initTemplateFromRule = (tpl) => {
    const t = normalizeTemplate(tpl);
    setTemplate(t);
    setPresetKey(detectPreset(t));
  };

  // 编辑 Segment 段配置单字段（index/keepPrefix/keepSuffix/maskMinLen）。
  const handleSegmentFieldChange = (segIdx, field, value) => {
    setTemplate((prev) => {
      if (!isSegmentTemplate(prev)) return prev;
      const segments = prev.segments.map((s, i) =>
        i === segIdx ? { ...s, [field]: value } : s,
      );
      return { ...prev, segments };
    });
  };

  // 新增一段段配置（默认 index=0, keepPrefix=1, keepSuffix=1, maskMinLen=1）。
  const handleAddSegment = () => {
    setTemplate((prev) => {
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

  // 删除指定段配置。
  const handleRemoveSegment = (segIdx) => {
    setTemplate((prev) => {
      if (!isSegmentTemplate(prev)) return prev;
      return {
        ...prev,
        segments: prev.segments.filter((_, i) => i !== segIdx),
      };
    });
  };

  const handleRuleChange = (id) => {
    const rule = maskRules.find((r) => r.id === id);
    if (!rule) return;
    setMaskRuleId(id);
    if ((id === "simple-mask" || id === "segment-mask") && rule.template) {
      initTemplateFromRule(rule.template);
    } else if (id === "name-mask") {
      const ch = resolveMaskChar(rule);
      setNameMaskChar(ch);
      form.setFieldValue("nameMaskChar", ch);
    }
  };

  // 选预设 → 填充 7 参数框（custom 不填充，保留当前用户输入）。
  const handlePresetChange = (key) => {
    setPresetKey(key);
    const filled = templateFromPreset(key);
    if (filled === null) return; // custom → 不填充
    setTemplate(filled);
  };

  // 编辑单个参数框 → 更新 template + 重新检测匹配的预设。
  const handleTemplateFieldChange = (field, value) => {
    setTemplate((prev) => {
      const next = { ...prev, [field]: value };
      setPresetKey(detectPreset(next));
      return next;
    });
  };

  // 组装 template（null 字段表示不设）。全空 → null（透传，用规则自身空模板）。
  const buildForRun = () => buildTemplateForRun(template);

  const handleRun = async () => {
    if (!sheet) {
      message.warning("请先导入数据");
      return;
    }
    const column = form.getFieldValue("column");
    if (!column) {
      message.warning("请选择要脱敏的列");
      return;
    }
    setLoading(true);
    try {
      let res;
      // v1.1.5 T85：先校验再脱敏参数。validateEnabled=false 时四个参数全部传 null，
      // 后端按旧逻辑直接脱敏。phone-validate 时透传前缀白名单（过滤三位数字）。
      const vRuleId = validateEnabled ? validateRuleId : null;
      const vInvalidText = validateEnabled ? invalidText : null;
      const vPhonePrefixes =
        validateEnabled && vRuleId === "phone-validate"
          ? (phonePrefixesInput || [])
              .map((p) => String(p).trim())
              .filter((p) => /^\d{3}$/.test(p))
          : null;
      // paramsOverride 目前仅 generic-validate 等可后续扩展；本面板不暴露行级参数，
      // 透传 null（后端沿用 DB 规则默认 params）。
      const vParamsOverride = null;
      if (isTemplateMask) {
        const tpl = buildForRun();
        // simple-mask/segment-mask：template 透传（临时覆盖），replacement 用 maskChar 或默认 *。
        const ch = template.maskChar || DEFAULT_MASK_CHAR;
        res = await maskColumn(
          sheet.id,
          column,
          maskRuleId,
          ch,
          tpl,
          vRuleId,
          vInvalidText,
          vParamsOverride,
          vPhonePrefixes,
        );
      } else {
        // name-mask：旧逻辑，无 template。
        const ch = nameMaskChar || DEFAULT_MASK_CHAR;
        res = await maskColumn(
          sheet.id,
          column,
          maskRuleId,
          ch,
          null,
          vRuleId,
          vInvalidText,
          vParamsOverride,
          vPhonePrefixes,
        );
      }
      const page = sheet.page || 1;
      const data = await getSheetData(sheet.id, page, sheet.pageSize || PAGE_SIZE);
      dispatch({
        type: "SET_SHEET_DATA",
        payload: { ...data, sheetId: sheet.id },
      });
      const rowStatuses = {};
      (data.rows || []).forEach((_, i) => {
        rowStatuses[`${sheet.id}-${page}-${i}`] = "masked";
      });
      applyRowStatuses({ sheetId: sheet.id, rowStatuses });
      message.success(`脱敏完成：${res.affected ?? 0} 行`);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("mask_column failed:", e);
      message.error(`脱敏失败：${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSaveSetting = async () => {
    if (!maskRuleId) {
      message.warning("未找到脱敏规则，无法保存");
      return;
    }
    setSaving(true);
    try {
      if (isTemplateMask) {
        // 持久化 template 到 simple-mask/segment-mask 规则（rules.template 列）。
        const tpl = buildForRun();
        await updateRuleTemplate(maskRuleId, tpl);
        message.success("脱敏参数已保存");
      } else {
        // name-mask：持久化掩码字符到 replacement。
        const ch = nameMaskChar || DEFAULT_MASK_CHAR;
        await updateRuleParams(maskRuleId, null, ch);
        message.success("掩码字符已保存");
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("save mask setting failed:", e);
      message.error(`保存失败：${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    if (isSimpleMask) {
      setTemplate({ ...EMPTY_TEMPLATE });
      setPresetKey("empty");
    } else if (isSegmentMask) {
      setTemplate({ ...EMPTY_SEGMENT_TEMPLATE });
      setPresetKey("custom");
    } else {
      setNameMaskChar(DEFAULT_MASK_CHAR);
      form.setFieldValue("nameMaskChar", DEFAULT_MASK_CHAR);
    }
    // v1.1.5 T85：同时重置先校验再脱敏开关与参数。
    setValidateEnabled(false);
    setInvalidText("INVALID");
    setPhonePrefixesInput([]);
    if (validateRules.length > 0) {
      setValidateRuleId(validateRules[0].id);
    }
  };

  return (
    <div style={{ padding: 4 }}>
      <Form form={form} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
          <Select
            placeholder="选择要脱敏的列"
            options={headers.map((h) => ({ label: h, value: h }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        <Form.Item
          label="脱敏规则"
          name="maskRuleId"
          extra="姓名脱敏（保留首尾）/ 整段脱敏（模板参数）/ 分段脱敏（按分隔符拆分）"
        >
          <Select
            placeholder="选择脱敏规则"
            value={maskRuleId}
            onChange={handleRuleChange}
            options={maskRules.map((r) => ({ label: r.name, value: r.id }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        {isSegmentMask ? (
          <>
            <Form.Item label="分隔符" extra="如 @ . - / 等单字符或多字符">
              <Input
                value={template.delimiter}
                onChange={(e) =>
                  setTemplate((prev) => ({ ...prev, delimiter: e.target.value }))
                }
                placeholder="@ / . 等"
                allowClear
              />
            </Form.Item>
            <Form.Item label="掩码字符" extra="默认 *；取首个字符">
              <Input
                value={template.maskChar ?? ""}
                onChange={(e) =>
                  setTemplate((prev) => ({
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
                {(template.segments || []).map((seg, i) => (
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
                            onChange={(v) => handleSegmentFieldChange(i, "index", v)}
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
        ) : isSimpleMask ? (
          <>
            <Form.Item
              label="子规则（预设）"
              extra="选预设填充参数，可继续修改；空模板=不脱敏（透传）"
            >
              <Select
                value={presetKey}
                onChange={handlePresetChange}
                options={MASK_PRESETS.map((p) => ({ label: p.label, value: p.key }))}
              />
            </Form.Item>
            <Form.Item label="保留前缀字符数">
              <InputNumber
                value={template.keepPrefix}
                onChange={(v) => handleTemplateFieldChange("keepPrefix", v)}
                placeholder="0"
                min={0}
                style={{ width: "100%" }}
              />
            </Form.Item>
            <Form.Item label="保留后缀字符数">
              <InputNumber
                value={template.keepSuffix}
                onChange={(v) => handleTemplateFieldChange("keepSuffix", v)}
                placeholder="0"
                min={0}
                style={{ width: "100%" }}
              />
            </Form.Item>
            <Form.Item label="掩码字符" extra="默认 *；取首个字符">
              <Input
                value={template.maskChar ?? ""}
                onChange={(e) =>
                  handleTemplateFieldChange("maskChar", e.target.value || null)
                }
                placeholder={DEFAULT_MASK_CHAR}
                allowClear
              />
            </Form.Item>
            <Form.Item label="最少掩码字符数">
              <InputNumber
                value={template.maskMinLen}
                onChange={(v) => handleTemplateFieldChange("maskMinLen", v)}
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
                checked={template.reverse === true}
                onChange={(checked) =>
                  handleTemplateFieldChange("reverse", checked || null)
                }
              />
            </Form.Item>
            <Form.Item label="值长度下限（guard，空=不限）">
              <InputNumber
                value={template.minLen}
                onChange={(v) => handleTemplateFieldChange("minLen", v)}
                placeholder="不限"
                min={0}
                style={{ width: "100%" }}
              />
            </Form.Item>
            <Form.Item label="值长度上限（guard，空=不限）">
              <InputNumber
                value={template.maxLen}
                onChange={(v) => handleTemplateFieldChange("maxLen", v)}
                placeholder="不限"
                min={0}
                style={{ width: "100%" }}
              />
            </Form.Item>
          </>
        ) : (
          <Form.Item label="掩码字符" name="nameMaskChar" extra="默认 *；取首个字符">
            <Input
              value={nameMaskChar}
              onChange={(e) => setNameMaskChar(e.target.value)}
              placeholder={DEFAULT_MASK_CHAR}
              allowClear
            />
          </Form.Item>
        )}
        {/* v1.1.5 T85：先校验再脱敏。勾选后先按 validate 规则校验该列，
            通过的行按 mask 规则脱敏；未通过行写为 invalidText 占位（默认 "INVALID"）。
            phone-validate 时额外展开前缀白名单输入（与 ValidatePanel 一致 tags 模式）。 */}
        <Form.Item
          extra={
            validateEnabled
              ? "校验通过的行按上方规则脱敏；未通过行写为占位文本"
              : "勾选后先按校验规则过滤该列，未通过行不脱敏"
          }
          style={{ marginTop: 8 }}
        >
          <Checkbox
            checked={validateEnabled}
            onChange={(e) => setValidateEnabled(e.target.checked)}
          >
            先校验再脱敏
          </Checkbox>
        </Form.Item>
        {validateEnabled ? (
          <>
            <Form.Item label="校验规则">
              {validateRules.length === 0 ? (
                <Empty
                  description="未找到 validate 规则"
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                />
              ) : (
                <Select
                  placeholder="选择校验规则"
                  value={validateRuleId}
                  onChange={setValidateRuleId}
                  options={validateRules.map((r) => ({ label: r.name, value: r.id }))}
                  showSearch
                  optionFilterProp="label"
                />
              )}
            </Form.Item>
            <Form.Item label="无效输出文本" extra="校验未通过行的占位文本（默认 INVALID）">
              <Input
                value={invalidText}
                onChange={(e) => setInvalidText(e.target.value)}
                placeholder="INVALID"
                allowClear
              />
            </Form.Item>
            {validateRuleId === "phone-validate" ? (
              <Form.Item
                label="手机号前缀白名单"
                extra="可选：填三位数字前缀（如 134 / 159），留空则不限制前缀"
              >
                <Select
                  mode="tags"
                  placeholder="如 134、159（回车添加）"
                  value={phonePrefixesInput}
                  onChange={setPhonePrefixesInput}
                  tokenSeparators={[",", "，"]}
                  maxTagCount={5}
                />
              </Form.Item>
            ) : null}
          </>
        ) : null}
        <Form.Item>
          <Space>
            <Button type="primary" loading={loading} onClick={handleRun}>
              执行脱敏
            </Button>
            <Button onClick={handleReset}>重置</Button>
            <Button loading={saving} onClick={handleSaveSetting}>
              保存设置
            </Button>
          </Space>
        </Form.Item>
      </Form>
    </div>
  );
}
