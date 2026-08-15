import { useEffect, useState } from "react";
import { Button, Checkbox, Empty, Form, Input, Select, Space, message } from "antd";
import { useAppContext, ACTION } from "../../state";
import {
  maskColumn,
  updateRuleParams,
  updateRuleTemplate,
} from "../../tauri";
import { INVALID_TEXT } from "../../constants";
import { useRules } from "../../hooks/useRules";
import { useSheetOps } from "../../hooks/useSheetOps";
import { useActiveSheet } from "../../hooks/useActiveSheet";
import { normalizePhonePrefixes } from "../../utils/phonePrefix";
import ColumnSelect from "../shared/ColumnSelect";
import PhonePrefixSelect from "../shared/PhonePrefixSelect";
import TemplateEditor from "../shared/TemplateEditor";
import {
  DEFAULT_MASK_CHAR,
  EMPTY_SEGMENT_TEMPLATE,
  EMPTY_TEMPLATE,
  buildTemplateForRun,
  detectPreset,
  normalizeTemplate,
  resolveMaskChar,
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
//   - v1.2.0 T94：useRules 替代内联 listRules useEffect（mask + validate 双调用），
//     ColumnSelect / PhonePrefixSelect 替代重复 Select。脱敏刷新逻辑保留内联
//     （需要 data.rows 做 applyRowStatuses，与通用 refreshActiveSheet 模式不同）。
export default function MaskPanel() {
  const { dispatch } = useAppContext();
  const { sheet, headers } = useActiveSheet();
  const { refreshActiveSheet } = useSheetOps(dispatch);
  const { rules: maskRules } = useRules("mask");
  const { rules: validateRules } = useRules("validate");
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [maskRuleId, setMaskRuleId] = useState(null);
  // simple-mask 的 7 个模板参数（camelCase，对齐后端 TemplateParams::Simple）
  // segment-mask 的分段参数（delimiter + segments[]，对齐 Segment 变体）
  const [presetKey, setPresetKey] = useState("empty");
  const [template, setTemplate] = useState({ ...EMPTY_TEMPLATE });
  // name-mask 的掩码字符（replacement）
  const [nameMaskChar, setNameMaskChar] = useState(DEFAULT_MASK_CHAR);
  // v1.1.5 T85：先校验再脱敏。勾选后先按 validate 规则过滤该列，通过的行脱敏，
  // 未通过行写为 invalidText 占位（默认 "INVALID"）。
  const [validateEnabled, setValidateEnabled] = useState(false);
  const [validateRuleId, setValidateRuleId] = useState(null);
  const [invalidText, setInvalidText] = useState(INVALID_TEXT);
  // phone-validate 行级前缀白名单（与 ValidatePanel 模式一致，tags 模式输入）。
  const [phonePrefixesInput, setPhonePrefixesInput] = useState([]);

  const isSimpleMask = maskRuleId === "simple-mask";
  const isSegmentMask = maskRuleId === "segment-mask";
  const isTemplateMask = isSimpleMask || isSegmentMask;

  // 从规则 template 初始化参数框（simple-mask/segment-mask 从 DB 读取的 template）。
  const initTemplateFromRule = (tpl) => {
    const t = normalizeTemplate(tpl);
    setTemplate(t);
    setPresetKey(detectPreset(t));
  };

  // v1.2.0 T94：rules 加载由 useRules 接管，这里只做首次自动选中 + 模板初始化。
  useEffect(() => {
    if (maskRules.length === 0 || maskRuleId) return;
    const first = maskRules[0];
    setMaskRuleId(first.id);
    form.setFieldValue("maskRuleId", first.id);
    if ((first.id === "simple-mask" || first.id === "segment-mask") && first.template) {
      initTemplateFromRule(first.template);
    } else if (first.id === "name-mask") {
      const ch = resolveMaskChar(first);
      setNameMaskChar(ch);
      form.setFieldValue("nameMaskChar", ch);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [maskRules]);

  // T85：validate 规则加载后默认选中首条。
  useEffect(() => {
    if (validateRules.length > 0 && !validateRuleId) {
      setValidateRuleId(validateRules[0].id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [validateRules]);

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
          ? normalizePhonePrefixes(phonePrefixesInput || [])
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
      // 刷新当前页数据 + 行状态（masked）+ 撤销栈（useSheetOps 统一封装）
      await refreshActiveSheet(sheet, "masked");
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
    setInvalidText(INVALID_TEXT);
    setPhonePrefixesInput([]);
    if (validateRules.length > 0) {
      setValidateRuleId(validateRules[0].id);
    }
  };

  return (
    <div style={{ height: "100%", overflow: "auto", padding: 4 }}>
      <Form form={form} layout="vertical" size="small">
        <Form.Item label="目标列" name="column">
          <ColumnSelect headers={headers} placeholder="选择列" />
        </Form.Item>
        <Form.Item
          label="脱敏规则"
          name="maskRuleId"
          extra="姓名 / 整段 / 分段"
        >
          <Select
            placeholder="选择规则"
            value={maskRuleId}
            onChange={handleRuleChange}
            options={maskRules.map((r) => ({ label: r.name, value: r.id }))}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
        {isSegmentMask ? (
          <TemplateEditor
            mode="segment"
            template={template}
            setTemplate={setTemplate}
            presetKey={presetKey}
            setPresetKey={setPresetKey}
          />
        ) : isSimpleMask ? (
          <TemplateEditor
            mode="simple"
            template={template}
            setTemplate={setTemplate}
            presetKey={presetKey}
            setPresetKey={setPresetKey}
          />
        ) : (
          <Form.Item label="掩码字符" name="nameMaskChar" extra="默认 *">
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
              ? "通过行脱敏，未通过行写占位文本"
              : "先校验该列，未通过行不脱敏"
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
                  description="无校验规则"
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
            <Form.Item label="无效输出文本" extra="未通过行的占位文本">
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
                extra="三位数字前缀，留空=不限"
              >
                <PhonePrefixSelect
                  value={phonePrefixesInput}
                  onChange={setPhonePrefixesInput}
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