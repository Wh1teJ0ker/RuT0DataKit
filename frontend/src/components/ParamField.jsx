import React from "react";
import { InputNumber, Switch, Select, Input, Tooltip, Typography } from "antd";
import { ExclamationCircleOutlined } from "@ant-design/icons";

const { TextArea } = Input;
const { Text } = Typography;

// 通用 typed 参数渲染器（v0.1.0 规则引擎重构）。
// 据 paramTypes[pName].type 渲染对应 antd 组件：
//   number  → InputNumber
//   switch  → Switch
//   select  → Select（options 来自 def）
//   regex   → TextArea + 内联正则校验反馈（红/绿提示）
//   text    → Input（或 TextArea 当 multiline:true）
// params/onChange 由父组件透传；size 控制 InputNumber/Input 尺寸。
//
// 值规范化：number 类型空值 → null（便于后端 Option 解析）；
// switch 类型接受 boolean；其他类型接受 string。
export default function ParamField({
  paramName,
  def,
  value,
  onChange,
  size = "small",
  width = 120,
}) {
  const meta = (def && def.paramTypes && def.paramTypes[paramName]) || {
    type: "text",
  };
  const doc = (def && def.paramDocs && def.paramDocs[paramName]) || paramName;
  const placeholder = doc;

  // regex 类型做即时正则校验反馈
  const regexState = useRegexValidation(meta.type === "regex" ? value : null);

  const commonProps = {
    size,
    style: { width },
    placeholder,
  };

  switch (meta.type) {
    case "number":
      return (
        <Tooltip title={doc} trigger="hover">
          <InputNumber
            {...commonProps}
            min={meta.min}
            max={meta.max}
            value={value == null || value === "" ? null : Number(value)}
            onChange={(v) => onChange(v == null ? null : Number(v))}
          />
        </Tooltip>
      );
    case "switch":
      return (
        <Tooltip title={doc} trigger="hover">
          <Switch
            size={size === "small" ? "small" : "default"}
            checked={!!value}
            onChange={(v) => onChange(v)}
          />
        </Tooltip>
      );
    case "select":
      return (
        <Select
          {...commonProps}
          value={value == null || value === "" ? meta.default : value}
          options={(meta.options || []).map((o) => ({ label: o, value: o }))}
          onChange={(v) => onChange(v)}
        />
      );
    case "regex":
      return (
        <div style={{ display: "flex", flexDirection: "column", width }}>
          <TextArea
            rows={2}
            size={size}
            style={{ width, fontFamily: "monospace", fontSize: 12 }}
            placeholder={doc}
            value={value == null ? "" : String(value)}
            onChange={(e) => onChange(e.target.value)}
          />
          {value ? (
            <Text
              style={{
                fontSize: 11,
                marginTop: 2,
                color: regexState.ok ? "#52c41a" : "#ff4d4f",
              }}
            >
              {regexState.ok ? "正则合法" : `正则错误: ${regexState.error}`}
            </Text>
          ) : null}
        </div>
      );
    case "text":
    default:
      if (meta.multiline) {
        return (
          <TextArea
            rows={2}
            size={size}
            style={{ width, fontFamily: "monospace", fontSize: 12 }}
            placeholder={doc}
            value={value == null ? "" : String(value)}
            onChange={(e) => onChange(e.target.value)}
          />
        );
      }
      return (
        <Tooltip title={doc} trigger="hover">
          <Input
            {...commonProps}
            value={value == null ? "" : String(value)}
            onChange={(e) => onChange(e.target.value)}
            maxLength={meta.maxLength}
          />
        </Tooltip>
      );
  }
}

// 内联正则校验 hook：仅在 type === "regex" 时启用。
// 用浏览器内置 RegExp 试编译；非法时返回 error 字符串。注意 Rust regex 语法
// 与 JS 正则有少量差异（如 (?P<name>...)），但常用 PCRE 子集兼容。
function useRegexValidation(pattern) {
  const [state, setState] = React.useState({ ok: true, error: "" });
  React.useEffect(() => {
    if (pattern == null || pattern === "") {
      setState({ ok: true, error: "" });
      return;
    }
    try {
      // eslint-disable-next-line no-new
      new RegExp(pattern);
      setState({ ok: true, error: "" });
    } catch (e) {
      setState({ ok: false, error: String(e).replace(/^Error: /, "") });
    }
  }, [pattern]);
  return state;
}

// 工具：从 def.paramTypes 提取默认 params 对象，用于新增规则时初始化。
export function buildDefaultParams(def) {
  if (!def || !def.paramTypes) return {};
  const out = {};
  for (const [k, meta] of Object.entries(def.paramTypes)) {
    if (meta.optional) continue;
    out[k] = meta.default != null ? meta.default : "";
  }
  return out;
}

// 工具：把表单 values 规范化为后端期望的 params 对象。
// number 类型空字符串 → null；switch → boolean；其他 → string。
export function normalizeParams(def, values) {
  if (!def || !def.paramTypes) return {};
  const out = {};
  for (const p of def.params || []) {
    const meta = def.paramTypes[p] || { type: "text" };
    const v = values[p];
    if (meta.type === "number") {
      out[p] = v == null || v === "" ? null : Number(v);
    } else if (meta.type === "switch") {
      out[p] = !!v;
    } else {
      out[p] = v == null ? "" : String(v);
    }
  }
  return out;
}
