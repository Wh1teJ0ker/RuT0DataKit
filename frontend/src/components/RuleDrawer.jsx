import React from "react";
import {
  Drawer,
  Form,
  Input,
  Select,
  Radio,
  Button,
  Space,
  Typography,
  Spin,
  Alert,
  Tag,
  Divider,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ArrowRightOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
} from "@ant-design/icons";
import { MASKER_DEFS, getMaskerDef } from "../maskerDefs.js";
import { VALIDATOR_DEFS, getValidatorDef } from "../validatorDefs.js";
import {
  listMaskOpTypes,
  listValidateOpTypes,
  listRuleTags,
  previewMaskRuleValue,
  previewValidateRuleValue,
} from "../tauri.js";
import ParamField, {
  buildDefaultParams,
  normalizeParams,
} from "./ParamField.jsx";
import FieldInput from "./FieldInput.jsx";

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

// 规则抽屉表单（v0.1.0 重构）：替代旧 RuleForm，新增/编辑同一入口。
// 关键优化：
// 1. 按 paramTypes 渲染 typed 输入（InputNumber/Switch/Select/TextArea regex
//    + 内联正则校验反馈）替代纯 Input 文本框；
// 2. 顶部「试运行」按钮调 preview_mask_rule/preview_validate_rule 对当前
//    表单值跑首行该 field，展示 input→output / pass/fail，保存前即时反馈；
// 3. op Select options 异步加载 listMaskOpTypes/listValidateOpTypes，失败
//    fallback 到本地 MASKER_DEFS/VALIDATOR_DEFS。
export default function RuleDrawer({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [form] = Form.useForm();
  const editingRule = state.editingRule;
  const open = !!editingRule;
  const isEdit = !!(editingRule && editingRule.__index != null);

  const [opTypes, setOpTypes] = React.useState({
    mask: MASKER_DEFS.map((m) => ({ name: m.name, label: m.description })),
    validate: VALIDATOR_DEFS.map((v) => ({ name: v.name, label: v.description })),
  });

  // 可选标签下拉源：来自 list_rule_tags 命令（预置标签 ∪ 当前 ruleset tag），
  // 合并当前编辑规则已有 tags（避免编辑态丢掉未在列表中的自定义 tag）。
  // rulesJson 取自 state.rules 序列化，编辑/新增态共用同一份下拉。
  const [tagOptions, setTagOptions] = React.useState([]);
  const rulesJson = React.useMemo(() => {
    const r = state.rules || { maskers: [], validators: [] };
    return JSON.stringify({ maskers: r.maskers, validators: r.validators });
  }, [state.rules]);
  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const tags = await listRuleTags(rulesJson);
        if (cancelled) return;
        setTagOptions(Array.isArray(tags) ? tags : []);
      } catch {
        if (!cancelled) setTagOptions([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [rulesJson]);
  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [maskList, validateList] = await Promise.all([
          listMaskOpTypes(),
          listValidateOpTypes(),
        ]);
        if (cancelled) return;
        const next = { ...opTypes };
        if (Array.isArray(maskList) && maskList.length)
          next.mask = maskList.map((x) => ({ name: x.name, label: x.label }));
        if (Array.isArray(validateList) && validateList.length)
          next.validate = validateList.map((x) => ({ name: x.name, label: x.label }));
        setOpTypes(next);
      } catch (e) {
        // eslint-disable-next-line no-console
        console.warn("list op types failed, fallback to local defs:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const kind = Form.useWatch("kind", form) || "mask";
  const opName = Form.useWatch("op", form);
  const def = kind === "mask" ? getMaskerDef(opName) : getValidatorDef(opName);

  const opOptions = (kind === "mask" ? opTypes.mask : opTypes.validate).map(
    (x) => ({ label: x.label || x.name, value: x.name })
  );
  // fieldOptions 仅作 FieldInput 候选；未导入数据时允许自由输入字段名。

  // editingRule 变化时回填表单。
  React.useEffect(() => {
    if (!editingRule) {
      form.resetFields();
      return;
    }
    if (editingRule.__index != null) {
      const k = editingRule.kind;
      const opVal = k === "mask" ? editingRule.masker : editingRule.validator;
      const curDef = k === "mask" ? getMaskerDef(opVal) : getValidatorDef(opVal);
      const values = {
        kind: k,
        field: editingRule.field,
        op: opVal,
        description: editingRule.description || "",
        tags: Array.isArray(editingRule.tags) ? [...editingRule.tags] : [],
      };
      for (const p of curDef.params) {
        const v =
          editingRule.params && editingRule.params[p] != null
            ? editingRule.params[p]
            : (curDef.paramTypes && curDef.paramTypes[p]?.default) ?? "";
        values[p] = v;
      }
      form.setFieldsValue(values);
    } else {
      // 新增态：选首个 op + 该 op 默认 params。
      form.resetFields();
      const k = editingRule.kind || "mask";
      const list = k === "mask" ? opTypes.mask : opTypes.validate;
      const firstOp = list.length ? list[0].name : undefined;
      const firstDef = k === "mask" ? getMaskerDef(firstOp) : getValidatorDef(firstOp);
      form.setFieldsValue({
        kind: k,
        op: firstOp,
        tags: [],
        ...buildDefaultParams(firstDef),
      });
    }
    // 切换 editingRule 时清空试运行结果
    setRun({ loading: false, result: null, error: null });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editingRule]);

  // 切换 op 时重置为新 op 的默认 params（保留 field/kind）。
  const handleOpChange = (newOp) => {
    const curDef = kind === "mask" ? getMaskerDef(newOp) : getValidatorDef(newOp);
    if (!curDef) return;
    const reset = {};
    // 清空旧 op 字段
    if (opName) {
      const oldDef = kind === "mask" ? getMaskerDef(opName) : getValidatorDef(opName);
      for (const p of oldDef.params) reset[p] = "";
    }
    for (const [p, v] of Object.entries(buildDefaultParams(curDef))) reset[p] = v;
    form.setFieldsValue(reset);
    setRun({ loading: false, result: null, error: null });
  };

  const handleKindChange = (e) => {
    const newKind = e.target.value;
    if (opName) {
      const oldDef = kind === "mask" ? getMaskerDef(opName) : getValidatorDef(opName);
      const cleared = {};
      for (const p of oldDef.params) cleared[p] = "";
      form.setFieldsValue(cleared);
    }
    const list = newKind === "mask" ? opTypes.mask : opTypes.validate;
    const first = list.length ? list[0].name : undefined;
    const firstDef =
      newKind === "mask" ? getMaskerDef(first) : getValidatorDef(first);
    form.setFieldsValue({ op: first, ...buildDefaultParams(firstDef) });
    setRun({ loading: false, result: null, error: null });
  };

  const handleClose = () => {
    dispatch({ type: "CLEAR_EDITING_RULE" });
  };

  const handleFinish = (values) => {
    const k = values.kind || "mask";
    const curDef = k === "mask" ? getMaskerDef(values.op) : getValidatorDef(values.op);
    const params = normalizeParams(curDef, values);
    const tags = Array.isArray(values.tags) ? values.tags.filter((t) => t && t.trim()).map((t) => t.trim()) : [];
    const rule = {
      field: values.field,
      description: values.description ? values.description : undefined,
      params,
      tags,
    };
    if (k === "mask") rule.masker = values.op;
    else rule.validator = values.op;

    if (isEdit) {
      dispatch({
        type: "UPDATE_RULE",
        kind: k,
        index: editingRule.__index,
        rule,
      });
    } else {
      dispatch({ type: "ADD_RULE", kind: k, rule });
    }
    dispatch({ type: "CLEAR_EDITING_RULE" });
  };

  // ── 试运行（独立输入样例值，不依赖任何文件） ─────────────────────
  const [sampleInput, setSampleInput] = React.useState("");
  const [run, setRun] = React.useState({
    loading: false,
    result: null,
    error: null,
  });
  // 规则管理是独立系统，试运行不再读取已导入数据文件；
  // 用户手动输入样例值即可，无需字段或文件。

  const handlePreview = async () => {
    // 仅校验 op；字段与样例值都允许为空（空样例值对算子跑空串）。
    try {
      await form.validateFields(["op", "kind"]);
    } catch {
      return;
    }
    const values = form.getFieldsValue(true);
    const k = values.kind || "mask";
    const curDef = k === "mask" ? getMaskerDef(values.op) : getValidatorDef(values.op);
    const params = normalizeParams(curDef, values);
    const input = sampleInput || "";
    setRun({ loading: true, result: null, error: null });
    try {
      let res;
      if (k === "mask") {
        res = await previewMaskRuleValue(input, values.op, params);
      } else {
        res = await previewValidateRuleValue(
          input,
          values.op,
          params,
          values.regex || null,
          values.message || null
        );
      }
      setRun({ loading: false, result: res, error: null });
    } catch (e) {
      setRun({ loading: false, result: null, error: String(e) });
    }
  };

  return (
    <Drawer
      width={520}
      open={open}
      onClose={handleClose}
      title={isEdit ? "编辑规则" : "添加规则"}
      destroyOnClose
      extra={
        <Space>
          <Button onClick={handleClose}>取消</Button>
          <Button type="primary" onClick={() => form.submit()}>
            保存
          </Button>
        </Space>
      }
    >
      <Form form={form} layout="vertical" initialValues={{ kind: "mask" }} onFinish={handleFinish}>
        <Form.Item label="类型" name="kind">
          <Radio.Group optionType="button" buttonStyle="solid" onChange={handleKindChange}>
            <Radio.Button value="mask">脱敏</Radio.Button>
            <Radio.Button value="validate">校验</Radio.Button>
          </Radio.Group>
        </Form.Item>

        <Form.Item
          label="字段"
          name="field"
          extra="规则独立于文件管理；可输入字段名或从已导入数据表头中选择"
        >
          <FieldInput
            headers={state.headers}
            placeholder="输入或选择字段名"
          />
        </Form.Item>

        <Form.Item label="算子" name="op" rules={[{ required: true, message: "请选择算子" }]}>
          <Select
            placeholder="选择算子"
            options={opOptions}
            notFoundContent="无可用算子"
            showSearch
            onChange={handleOpChange}
          />
        </Form.Item>

        {def ? (
          <div style={{ marginBottom: 8 }}>
            <Paragraph type="secondary" style={{ fontSize: 12, margin: 0 }}>
              {def.description}
            </Paragraph>
          </div>
        ) : null}

        <Form.Item
          label="标签"
          name="tags"
          extra="可选：规则分组标签，供按标签过滤；可直接输入自定义标签"
        >
          <Select
            mode="tags"
            placeholder="选择或输入标签（如 mask / sensitive）"
            options={tagOptions.map((t) => ({ label: t, value: t }))}
            tokenSeparators={[",", " "]}
            style={{ width: "100%" }}
          />
        </Form.Item>

        {def && def.params.length === 0 ? (
          <Text type="secondary" style={{ fontSize: 12 }}>
            该算子无参数
          </Text>
        ) : (
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "1fr 1fr",
              gap: "8px 12px",
            }}
          >
            {def
              ? def.params.map((p) => {
                  const meta = def.paramTypes?.[p] || { type: "text" };
                  // regex / 多行占满整行
                  const fullWidth = meta.type === "regex" || meta.multiline;
                  return (
                    <div
                      key={p}
                      style={{ gridColumn: fullWidth ? "1 / -1" : "auto" }}
                    >
                      <div style={{ fontSize: 12, color: "#666", marginBottom: 2 }}>
                        {def.paramDocs?.[p] || p}
                        {meta.optional ? (
                          <Text type="secondary" style={{ fontSize: 11, marginLeft: 4 }}>
                            （可选）
                          </Text>
                        ) : null}
                      </div>
                      <Form.Item name={p} style={{ marginBottom: 0 }}>
                        <ParamField def={def} paramName={p} width="100%" />
                      </Form.Item>
                    </div>
                  );
                })
              : null}
          </div>
        )}

        <Divider style={{ margin: "12px 0" }} />

        <Form.Item label="描述" name="description">
          <TextArea placeholder="可选：规则说明" autoSize={{ minRows: 2, maxRows: 4 }} />
        </Form.Item>
      </Form>

      {/* 试运行面板：用户手动输入样例值，不依赖任何文件 */}
      <div
        style={{
          marginTop: 12,
          padding: 12,
          background: "#fafafa",
          borderRadius: 6,
        }}
      >
        <Space size="small" style={{ marginBottom: 8 }} align="center">
          <Text strong style={{ fontSize: 13 }}>
            试运行
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            输入一个样例值，对当前算子跑一次（不依赖任何文件）
          </Text>
        </Space>
        <Space size="small" wrap align="center" style={{ width: "100%" }}>
          <Input
            placeholder="输入样例值，如 13812345678"
            value={sampleInput}
            onChange={(e) => setSampleInput(e.target.value)}
            onPressEnter={handlePreview}
            style={{ width: 280 }}
          />
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={run.loading}
            onClick={handlePreview}
          >
            试运行
          </Button>
        </Space>

        {run.loading ? (
          <div style={{ marginTop: 8 }}>
            <Spin size="small" />
          </div>
        ) : run.error ? (
          <Alert
            type="error"
            message="试运行失败"
            description={run.error}
            style={{ marginTop: 8 }}
          />
        ) : run.result ? (
          <div style={{ marginTop: 8 }}>
            {kind === "mask" ? (
              <Space size="small" wrap>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  输入
                </Text>
                <Text code style={{ fontSize: 12 }}>
                  {run.result.input}
                </Text>
                <ArrowRightOutlined style={{ fontSize: 12, color: "#999" }} />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  输出
                </Text>
                <Text code style={{ fontSize: 12 }}>
                  {run.result.output}
                </Text>
              </Space>
            ) : (
              <Space size="small" wrap align="center">
                <Tag
                  color={run.result.valid ? "green" : "red"}
                  icon={
                    run.result.valid ? (
                      <CheckCircleOutlined />
                    ) : (
                      <CloseCircleOutlined />
                    )
                  }
                  style={{ margin: 0 }}
                >
                  {run.result.valid ? "通过" : "失败"}
                </Tag>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  输入
                </Text>
                <Text code style={{ fontSize: 12 }}>
                  {run.result.input}
                </Text>
                {run.result.message ? (
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {run.result.message}
                  </Text>
                ) : null}
              </Space>
            )}
          </div>
        ) : null}
      </div>
    </Drawer>
  );
}
