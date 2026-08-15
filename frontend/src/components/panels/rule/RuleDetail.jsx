import { Button, Card, Col, Form, Input, Row, Space, Switch, Tag, Typography } from "antd";
import TemplateEditor from "../../shared/TemplateEditor";
import {
  DEFAULT_MASK_CHAR,
  normalizeTemplate,
  detectPreset,
} from "../maskTemplate";
import {
  EMPTY_GENERIC_PARAMS,
  normalizeGenericParams,
  buildGenericParamsForRun,
  VALIDATE_HINTS,
} from "../validateParams";
import { BUILTIN_PATTERNS, KIND_COLOR, KIND_LABEL, VALIDATE_LABELS } from "./constants";
import PhonePrefixParams from "./params/PhonePrefixParams";
import GenericParams from "./params/GenericParams";
import IdcardParams from "./params/IdcardParams";
import PatternParams from "./params/PatternParams";

const { Text } = Typography;

// 规则详情：规则信息 Card + 可填参数 Card（含保存/重置按钮）。
// 草稿态由父组件 RulesPanel 管理，通过 props 下发 + 回调上提。
export default function RuleDetail({
  selected,
  draftPattern,
  setDraftPattern,
  draftReplacement,
  setDraftReplacement,
  draftTemplate,
  setDraftTemplate,
  presetKey,
  setPresetKey,
  draftAllowedPrefixes,
  setDraftAllowedPrefixes,
  draftAllowLeadingZero,
  setDraftAllowLeadingZero,
  draftGenericParams,
  setDraftGenericParams,
  saving,
  onSave,
  onToggle,
}) {
  const handleReset = () => {
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
        selected.replacement && selected.replacement.length > 0
          ? selected.replacement
          : DEFAULT_MASK_CHAR
      );
    } else if (selected.params?.validator === "phonePrefix") {
      // 重置 = 恢复出厂正则 + 空前缀列表（不过滤前缀）。
      setDraftPattern(BUILTIN_PATTERNS[selected.id] ?? "");
      setDraftAllowedPrefixes([]);
    } else if (selected.params?.validator === "generic") {
      // v1.1.4 续轮 T71：generic-validate 重置 → 用 DB params 回填。
      setDraftGenericParams(normalizeGenericParams(selected.params));
    } else {
      // 提取规则（非 phonePrefix）重置为出厂正则；
      // 其他 validate 规则恢复 DB pattern。
      setDraftPattern(
        BUILTIN_PATTERNS[selected.id] ?? selected.pattern ?? ""
      );
      // idcard-extract 重置 → 恢复首位非零出厂状态。
      setDraftAllowLeadingZero(false);
    }
  };

  return (
    <div style={{ padding: "16px 24px" }}>
      <Typography.Title level={4} style={{ marginBottom: 4 }}>
        {selected.name}
      </Typography.Title>

      <Card size="small" title="规则信息" style={{ marginBottom: 16 }}>
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
              onChange={onToggle}
            />
          </Col>
        </Row>
      </Card>

      {/* 可填参数 */}
      <Card size="small" title="可填参数" style={{ marginBottom: 16 }}>
        <Form layout="vertical" size="small">
          {renderParams({
            selected,
            draftPattern,
            setDraftPattern,
            draftReplacement,
            setDraftReplacement,
            draftTemplate,
            setDraftTemplate,
            presetKey,
            setPresetKey,
            draftAllowedPrefixes,
            setDraftAllowedPrefixes,
            draftAllowLeadingZero,
            setDraftAllowLeadingZero,
            draftGenericParams,
            setDraftGenericParams,
          })}
          <Space>
            <Button type="primary" loading={saving} onClick={onSave}>
              保存参数
            </Button>
            <Button onClick={handleReset}>重置</Button>
          </Space>
        </Form>
      </Card>
    </div>
  );
}

// 按 selected.kind + selected.params 分支渲染可填参数。
function renderParams({
  selected,
  draftPattern,
  setDraftPattern,
  draftReplacement,
  setDraftReplacement,
  draftTemplate,
  setDraftTemplate,
  presetKey,
  setPresetKey,
  draftAllowedPrefixes,
  setDraftAllowedPrefixes,
  draftAllowLeadingZero,
  setDraftAllowLeadingZero,
  draftGenericParams,
  setDraftGenericParams,
}) {
  // mask 规则
  if (selected.kind === "mask") {
    if (selected.id === "segment-mask") {
      return (
        <TemplateEditor
          mode="segment"
          template={draftTemplate}
          setTemplate={setDraftTemplate}
          presetKey={presetKey}
          setPresetKey={setPresetKey}
        />
      );
    }
    if (selected.id === "simple-mask") {
      return (
        <TemplateEditor
          mode="simple"
          template={draftTemplate}
          setTemplate={setDraftTemplate}
          presetKey={presetKey}
          setPresetKey={setPresetKey}
        />
      );
    }
    return (
      <Form.Item label="掩码字符" extra="默认 *，保留首尾">
        <Input
          value={draftReplacement ?? ""}
          onChange={(e) => setDraftReplacement(e.target.value)}
          placeholder={DEFAULT_MASK_CHAR}
        />
      </Form.Item>
    );
  }

  // extract/validate 规则带 params
  if ((selected.kind === "extract" || selected.kind === "validate") && selected.params) {
    const v = selected.params.validator;
    if (v === "generic") {
      return (
        <GenericParams
          draftGenericParams={draftGenericParams}
          setDraftGenericParams={setDraftGenericParams}
          hint={VALIDATE_HINTS["generic-validate"] || ""}
        />
      );
    }
    if (v === "phonePrefix") {
      return (
        <PhonePrefixParams
          draftPattern={draftPattern}
          setDraftPattern={setDraftPattern}
          draftAllowedPrefixes={draftAllowedPrefixes}
          setDraftAllowedPrefixes={setDraftAllowedPrefixes}
        />
      );
    }
    if (v === "idcard" && selected.kind === "extract") {
      return (
        <IdcardParams
          draftPattern={draftPattern}
          setDraftPattern={setDraftPattern}
          draftAllowLeadingZero={draftAllowLeadingZero}
          setDraftAllowLeadingZero={setDraftAllowLeadingZero}
        />
      );
    }
    // 其他 validator（luhn/ipv4/ipv6/username/sex/birth/address）
    return (
      <PatternParams
        selected={selected}
        draftPattern={draftPattern}
        setDraftPattern={setDraftPattern}
        hint={VALIDATE_HINTS[selected.id] || ""}
      />
    );
  }

  // 无 params 的 validate 规则（如 name-validate）
  return (
    <PatternParams
      selected={selected}
      draftPattern={draftPattern}
      setDraftPattern={setDraftPattern}
      hint={VALIDATE_HINTS[selected.id] || ""}
    />
  );
}
