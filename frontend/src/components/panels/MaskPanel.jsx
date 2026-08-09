import { useEffect, useState } from "react";
import { Button, Form, Input, InputNumber, Select, Space, message } from "antd";
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
  EMPTY_TEMPLATE,
  MASK_PRESETS,
  buildTemplateForRun,
  detectPreset,
  normalizeTemplate,
  resolveMaskChar,
  templateFromPreset,
} from "./maskTemplate";

// v1.1.3 T49 脱敏面板：
//   - 脱敏规则下拉：姓名脱敏（name-mask）+ 通用脱敏（general-mask）。
//     4 条原独立规则（身份证/手机/出生日期/银行卡）收敛为 general-mask 的**预设**
//     （子规则），不再单独列出。
//   - 选 general-mask → 显示「子规则（预设）」下拉 + 6 个可编辑参数框。
//     选预设即填充参数，用户可继续修改；空模板（不选/全清）= 不脱敏（透传）。
//   - 选 name-mask → 仅显示掩码字符输入（旧逻辑，保留首尾各 1）。
//   - 执行脱敏：general-mask 把当前 6 参数框组装成 template 透传给 mask_column
//     （临时覆盖，不写回 DB）；name-mask 走旧逻辑。
//   - 保存设置：general-mask → updateRuleTemplate 持久化 template；name-mask →
//     updateRuleParams 持久化 replacement。
//   - T51：预设表 / 模板组装 / 透传判定抽到 ./maskTemplate.js，与 RulesPanel 共享。

export default function MaskPanel() {
  const { state, dispatch, applyRowStatuses } = useAppContext();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [maskRuleId, setMaskRuleId] = useState(null);
  const [maskRules, setMaskRules] = useState([]);
  // general-mask 的 6 个模板参数（camelCase，对齐后端 TemplateParams）
  const [presetKey, setPresetKey] = useState("empty");
  const [template, setTemplate] = useState({ ...EMPTY_TEMPLATE });
  // name-mask 的掩码字符（replacement）
  const [nameMaskChar, setNameMaskChar] = useState(DEFAULT_MASK_CHAR);

  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  const headers = sheet?.headers || [];
  const isGeneralMask = maskRuleId === "general-mask";

  // 加载全部 mask 规则供下拉选择；默认选中首条 mask 规则。
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const all = await listRules();
        const masks = all.filter((r) => r.kind === "mask");
        if (cancelled) return;
        setMaskRules(masks);
        if (masks.length === 0) return;
        const first = masks[0];
        setMaskRuleId(first.id);
        form.setFieldValue("maskRuleId", first.id);
        if (first.id === "general-mask" && first.template) {
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

  // 从规则 template 初始化参数框（general-mask 从 DB 读取的 template）。
  const initTemplateFromRule = (tpl) => {
    const t = normalizeTemplate(tpl);
    setTemplate(t);
    setPresetKey(detectPreset(t));
  };

  const handleRuleChange = (id) => {
    const rule = maskRules.find((r) => r.id === id);
    if (!rule) return;
    setMaskRuleId(id);
    if (id === "general-mask" && rule.template) {
      initTemplateFromRule(rule.template);
    } else if (id === "name-mask") {
      const ch = resolveMaskChar(rule);
      setNameMaskChar(ch);
      form.setFieldValue("nameMaskChar", ch);
    }
  };

  // 选预设 → 填充 6 参数框（custom 不填充，保留当前用户输入）。
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
      if (isGeneralMask) {
        const tpl = buildForRun();
        // general-mask：template 透传（临时覆盖），replacement 用 maskChar 或默认 *。
        const ch = template.maskChar || DEFAULT_MASK_CHAR;
        res = await maskColumn(sheet.id, column, maskRuleId, ch, tpl);
      } else {
        // name-mask：旧逻辑，无 template。
        const ch = nameMaskChar || DEFAULT_MASK_CHAR;
        res = await maskColumn(sheet.id, column, maskRuleId, ch, null);
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
      if (isGeneralMask) {
        // 持久化 template 到 general-mask 规则（rules.template 列）。
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
    if (isGeneralMask) {
      setTemplate({ ...EMPTY_TEMPLATE });
      setPresetKey("empty");
    } else {
      setNameMaskChar(DEFAULT_MASK_CHAR);
      form.setFieldValue("nameMaskChar", DEFAULT_MASK_CHAR);
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
          extra="姓名脱敏（保留首尾）或通用脱敏（模板参数）"
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
        {isGeneralMask ? (
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
