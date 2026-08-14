import { Button, Card, Form, Input, InputNumber, Row, Col, Select, Space, Switch } from "antd";
import {
  DEFAULT_MASK_CHAR,
  EMPTY_SEGMENT_TEMPLATE,
  EMPTY_TEMPLATE,
  MASK_PRESETS,
  detectPreset,
  isSegmentTemplate,
  templateFromPreset,
} from "../panels/maskTemplate";

/**
 * TemplateEditor — 共享脱敏模板参数编辑器。
 *
 * MaskPanel 和 RulesPanel 各有一份完全相同的 simple-mask / segment-mask
 * 模板编辑 UI（预设下拉 + 7 个参数框 / 分隔符 + 段配置 Card 列表）。
 * 本组件将其收敛为单一实现，消除 ~200 行 × 2 的重复代码。
 *
 * 受控模式：父组件持有 `template` / `presetKey` state + setter，
 * 本组件内部处理所有字段变更逻辑（预设填充、自动检测、段增删）。
 *
 * v1.2.0 T95：从 MaskPanel + RulesPanel 的模板编辑 UI 中抽出。
 *
 * @param {"simple"|"segment"} mode — 模板类型
 * @param {object} template — 当前模板对象（Simple 或 Segment 变体）
 * @param {Function} setTemplate — setTemplate(prev => next) 函数
 * @param {string} presetKey — 当前预设 key（仅 simple 模式用）
 * @param {Function} setPresetKey — setPresetKey(key) 函数
 */
export default function TemplateEditor({
  mode,
  template,
  setTemplate,
  presetKey,
  setPresetKey,
}) {
  // 选预设 → 填充参数框（custom 不填充，保留当前用户输入）。
  const handlePresetChange = (key) => {
    setPresetKey(key);
    const filled = templateFromPreset(key);
    if (filled === null) return; // custom → 不填充
    setTemplate(filled);
  };

  // 编辑单个参数框 → 更新 template + 重新检测匹配的预设。
  const handleFieldChange = (field, value) => {
    setTemplate((prev) => {
      const next = { ...prev, [field]: value };
      setPresetKey(detectPreset(next));
      return next;
    });
  };

  // 编辑 Segment 段配置单字段。
  const handleSegmentFieldChange = (segIdx, field, value) => {
    setTemplate((prev) => {
      if (!isSegmentTemplate(prev)) return prev;
      const segments = prev.segments.map((s, i) =>
        i === segIdx ? { ...s, [field]: value } : s
      );
      return { ...prev, segments };
    });
  };

  // 新增段配置。
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

  // 删除段配置。
  const handleRemoveSegment = (segIdx) => {
    setTemplate((prev) => {
      if (!isSegmentTemplate(prev)) return prev;
      return {
        ...prev,
        segments: prev.segments.filter((_, i) => i !== segIdx),
      };
    });
  };

  if (mode === "segment") {
    return (
      <>
        <Form.Item label="分隔符" extra="如 @ . - /">
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
                  <Col xs={{ span: 12 }} md={{ span: 6 }}>
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
                  <Col xs={{ span: 12 }} md={{ span: 6 }}>
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
                  <Col xs={{ span: 12 }} md={{ span: 6 }}>
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
                  <Col xs={{ span: 12 }} md={{ span: 6 }}>
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
    );
  }

  // mode === "simple"
  return (
    <>
      <Form.Item
        label="子规则（预设）"
        extra="选预设填充参数，空模板=不脱敏"
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
          onChange={(v) => handleFieldChange("keepPrefix", v)}
          placeholder="0"
          min={0}
          style={{ width: "100%" }}
        />
      </Form.Item>
      <Form.Item label="保留后缀字符数">
        <InputNumber
          value={template.keepSuffix}
          onChange={(v) => handleFieldChange("keepSuffix", v)}
          placeholder="0"
          min={0}
          style={{ width: "100%" }}
        />
      </Form.Item>
      <Form.Item label="掩码字符" extra="默认 *；取首个字符">
        <Input
          value={template.maskChar ?? ""}
          onChange={(e) =>
            handleFieldChange("maskChar", e.target.value || null)
          }
          placeholder={DEFAULT_MASK_CHAR}
          allowClear
        />
      </Form.Item>
      <Form.Item label="最少掩码字符数">
        <InputNumber
          value={template.maskMinLen}
          onChange={(v) => handleFieldChange("maskMinLen", v)}
          placeholder="1"
          min={0}
          style={{ width: "100%" }}
        />
      </Form.Item>
      <Form.Item
        label="反向脱敏"
        extra="保留中间，对首尾 N 位脱敏"
      >
        <Switch
          size="small"
          checked={template.reverse === true}
          onChange={(checked) =>
            handleFieldChange("reverse", checked || null)
          }
        />
      </Form.Item>
    </>
  );
}

// 导出空模板常量，供父组件初始化用。
export { EMPTY_TEMPLATE, EMPTY_SEGMENT_TEMPLATE };
