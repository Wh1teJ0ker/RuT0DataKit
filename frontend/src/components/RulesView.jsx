import React from "react";
import {
  Button,
  Card,
  Empty,
  Input,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message,
} from "antd";
import {
  ExperimentOutlined,
  RocketOutlined,
} from "@ant-design/icons";
import { listRuleTags, trialMask, trialValidate } from "../tauri.js";

const { Text } = Typography;

// tag -> 颜色映射：数据提取/数据脱敏/数据校验三选一，固定配色。
const TAG_COLORS = {
  extract: "cyan",
  mask: "blue",
  validate: "gold",
};

function tagColor(tag) {
  return TAG_COLORS[tag] ?? "default";
}

const TAG_LABELS = {
  extract: "数据提取",
  mask: "数据脱敏",
  validate: "数据校验",
};

function tagLabel(tag) {
  return TAG_LABELS[tag] ?? tag;
}

// 各脱敏模版（scope）的可填参数元信息：key + 中文 label + 类型。
// 用于动态渲染参数表单，空字符串值在提交时不写入 params（让后端默认生效）。
const MASK_PARAM_META = {
  template: [
    { key: "keep_prefix", label: "保留前 N 位", type: "number" },
    { key: "keep_suffix", label: "保留后 M 位", type: "number" },
    { key: "mask_char", label: "掩码字符", type: "char" },
    { key: "mask_min_len", label: "最少掩码长度", type: "number" },
    { key: "min_len", label: "最小长度 guard", type: "number" },
    { key: "max_len", label: "最大长度 guard", type: "number" },
    { key: "cjk", label: "中文姓名分支", type: "bool" },
  ],
  split_template: [
    { key: "separator", label: "分隔符", type: "char" },
    { key: "segment_index", label: "目标段索引（0 起）", type: "number" },
    { key: "keep_prefix", label: "段内保留前 N 位", type: "number" },
    { key: "keep_suffix", label: "段内保留后 M 位", type: "number" },
    { key: "mask_char", label: "掩码字符", type: "char" },
    { key: "mask_min_len", label: "最少掩码长度", type: "number" },
  ],
  regex_replace: [
    { key: "pattern", label: "正则 pattern", type: "text" },
    { key: "replacement", label: "替换串", type: "text" },
    { key: "match_mode", label: "匹配模式 (all/first)", type: "text" },
  ],
  const_replace: [
    { key: "with", label: "替换常量", type: "text" },
  ],
};

// 各校验模版（scope）的可填参数元信息（v0.6.2 新增，与 MASK_PARAM_META 对称）。
// regex：用户填 pattern（正则）+ 可选 message（失败消息）+ 可选 empty_message（空值失败消息）。
// v0.6.8（修订）：phone（extract）与 pinfo_phone（validate）统一支持 prefixes
// （前 1-3 位号段列表，逗号分隔输入 → 数组）。Rust 端 PhoneValidator::new 读
// params.prefixes（Value::Sequence 字符串序列），缺省/空走默认 1 开头正常号码
// （上一轮 v0.6.8 默认 52 虚假号段的设计不正确，已废弃）。前端用 list 类型，
// buildParams 保留非空数组。
const VALIDATE_PARAM_META = {
  regex: [
    { key: "pattern", label: "正则 pattern", type: "text" },
    { key: "message", label: "失败消息", type: "text" },
    { key: "empty_message", label: "空值失败消息", type: "text" },
  ],
  // v0.6.8（修订）：phone scope 用于数据提取规则，支持自定义前三位号段。
  phone: [
    {
      key: "prefixes",
      label: "前 1-3 位号段（逗号分隔，留空=默认 1 开头正常号码）",
      type: "list",
      placeholder: "138,159,734",
    },
  ],
  pinfo_phone: [
    {
      key: "prefixes",
      label: "前 1-3 位号段（逗号分隔，留空=默认 1 开头正常号码）",
      type: "list",
      placeholder: "138,159,734",
    },
  ],
};

// 按 scope 查参数元信息：先查脱敏模版，再查校验模版，否则返回空（无参数）。
function getParamMeta(scope) {
  return MASK_PARAM_META[scope] ?? VALIDATE_PARAM_META[scope] ?? [];
}

// 单条脱敏参数输入控件（行内展开区使用）。
function renderParamInput(meta, params, setParams) {
  const { key, label, type } = meta;
  if (type === "bool") {
    return (
      <label key={key} style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12 }}>
        <input
          type="checkbox"
          checked={!!params[key]}
          onChange={(e) => setParams((p) => ({ ...p, [key]: e.target.checked }))}
        />
        {label}
      </label>
    );
  }
  if (type === "text") {
    return (
      <label key={key} style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 12, minWidth: 160 }}>
        {label}
        <Input
          size="small"
          value={params[key] ?? ""}
          onChange={(e) => setParams((p) => ({ ...p, [key]: e.target.value }))}
          placeholder={key === "match_mode" ? "all" : ""}
        />
      </label>
    );
  }
  if (type === "char") {
    return (
      <label key={key} style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 12, width: 80 }}>
        {label}
        <Input
          size="small"
          maxLength={1}
          value={params[key] ?? ""}
          onChange={(e) => setParams((p) => ({ ...p, [key]: e.target.value.slice(0, 1) }))}
          placeholder={key === "mask_char" ? "*" : key === "separator" ? "@" : ""}
        />
      </label>
    );
  }
  if (type === "list") {
    // v0.6.8 T23-1：list 类型支持逗号分隔字符串输入 → 内部存为 string[]。
    // 显示时数组转逗号串；onChange 时 split + trim + 过滤空串 → 数组。
    const text = Array.isArray(params[key]) ? params[key].join(",") : (params[key] ?? "");
    return (
      <label key={key} style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 12, minWidth: 240 }}>
        {label}
        <Input.TextArea
          size="small"
          autoSize={{ minRows: 2, maxRows: 4 }}
          value={text}
          onChange={(e) => {
            const arr = e.target.value
              .split(",")
              .map((s) => s.trim())
              .filter((s) => s.length > 0);
            setParams((p) => ({ ...p, [key]: arr }));
          }}
          placeholder={meta.placeholder ?? ""}
        />
      </label>
    );
  }
  // number
  return (
    <label key={key} style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 12, width: 90 }}>
      {label}
      <Input
        size="small"
        type="number"
        value={params[key] ?? ""}
        onChange={(e) => {
          const v = e.target.value;
          setParams((p) => ({ ...p, [key]: v === "" ? "" : Number(v) }));
        }}
      />
    </label>
  );
}

// 操作列与展开区共享的 params 清洗：过滤空串/未勾选布尔，保留数值 0。
// v0.6.8 T23-1：list 类型保留非空数组、过滤空数组（留空 → 不写 params → 走后端默认）。
function buildParams(scope, params = {}) {
  const paramMeta = getParamMeta(scope);
  const cleaned = {};
  for (const { key, type } of paramMeta) {
    const v = params[key];
    if (type === "bool") {
      if (v) cleaned[key] = true;
      continue;
    }
    if (type === "list") {
      if (Array.isArray(v) && v.length > 0) cleaned[key] = v;
      continue;
    }
    if (v === undefined || v === "" || v === null) continue;
    cleaned[key] = v;
  }
  return cleaned;
}

// 行内展开区：mask / validate / extract 规则渲染参数表单 + 样例值 + 运行结果；其它规则渲染描述全文。
// 状态由 RulesView 通过 rowStates[key] + setRowState 下发，保证操作列按钮与表单共享。
function RowExpanded({ record, rowState, setRowState, state, dispatch, onRun, onApply }) {
  const paramMeta = getParamMeta(record.scope);
  // 无参数元信息的规则（如通用 validate 不带可编辑参数）只显示描述全文。
  // v0.6.8（修订）：phone（extract）与 pinfo_phone（validate）均有 prefixes 参数，
  // 会走下方的参数表单分支。
  if (paramMeta.length === 0) {
    return (
      <Text type="secondary" style={{ fontSize: 12 }}>
        {record.description || "（无描述）"}
      </Text>
    );
  }

  const isMaskRow = record.__kind === "mask" && record.tag === "mask";
  const isValidateRow = record.__kind === "validate" && record.tag === "validate";
  const isExtractRow = record.__kind === "validate" && record.tag === "extract";
  const { params = {}, sample = "", field = "", result = null } = rowState || {};

  const setParams = (updater) => {
    setRowState(record.key, (s) => ({ ...s, params: updater(s?.params ?? {}) }));
  };

  const headerOptions = React.useMemo(() => {
    const headers = state?.records?.headers || [];
    return headers.map((h) => ({ label: h, value: h }));
  }, [state?.records?.headers]);

  // 结果区文案随规则类型变化：mask 显示脱敏结果，validate/extract 显示合法/非法（匹配/不匹配）。
  // v0.6.8.1 修复：刚展开行时 rowState.result 为 null，原代码直接读 result.valid 会抛
  // TypeError: Cannot read properties of null (reading 'valid')，导致整个 RowExpanded
  // 渲染崩溃、展开区空白（用户所见「点击手机号出现空白」即此根因）。改用可选链，
  // result 为 null 时 validText 退回空串；result 在下方条件渲染中仍会跳过结果区。
  const applyLabel = isMaskRow
    ? "应用并跳转数据脱敏"
    : isValidateRow
      ? "应用并跳转数据校验"
      : isExtractRow
        ? "应用并跳转数据提取"
        : "应用";
  const resultLabel = isMaskRow ? "脱敏结果：" : isExtractRow ? "匹配结果：" : "校验结果：";
  const validText = isExtractRow
    ? (result?.valid ? "匹配 ✓" : "不匹配 ✗")
    : (result?.valid ? "合法 ✓" : "非法 ✗");

  return (
    <div style={{ padding: "4px 0" }}>
      {paramMeta.length === 0 ? (
        <Text type="secondary" style={{ fontSize: 12 }}>该模版无参数。</Text>
      ) : (
        <Space size="middle" wrap align="end" style={{ marginBottom: 8 }}>
          {paramMeta.map((m) => renderParamInput(m, params, setParams))}
        </Space>
      )}
      <Space size="middle" wrap align="end" style={{ marginBottom: 8 }}>
        <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 12, minWidth: 180 }}>
          应用字段名（CSV 表头）
          <Select
            size="small"
            showSearch
            placeholder={headerOptions.length ? "选择表头字段" : "请先导入文件"}
            value={field || undefined}
            onChange={(v) => {
              setRowState(record.key, (s) => ({ ...s, field: v }));
              // 顺带用该列首行的值预填样例，方便一键试运行。
              const rows = state?.records?.rows || [];
              const headers = state?.records?.headers || [];
              const colIdx = headers.indexOf(v);
              if (colIdx >= 0 && rows.length > 0) {
                const firstVal = rows[0]?.[colIdx];
                if (firstVal != null && firstVal !== "") {
                  setRowState(record.key, (s) => ({ ...s, sample: String(firstVal) }));
                }
              }
            }}
            options={headerOptions}
            style={{ width: "100%" }}
            notFoundContent="未导入文件"
          />
        </label>
        <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 12, minWidth: 240 }}>
          样例值
          <Input
            size="small"
            value={sample}
            onChange={(e) => setRowState(record.key, (s) => ({ ...s, sample: e.target.value }))}
            placeholder="如：张三 / 110101199005151234 / 13812345678"
            onPressEnter={() => onRun(record)}
          />
        </label>
        <Button size="small" type="primary" icon={<ExperimentOutlined />} loading={!!rowState?.loading} onClick={() => onRun(record)}>
          运行
        </Button>
        <Button size="small" icon={<RocketOutlined />} loading={!!rowState?.applying} onClick={() => onApply(record)}>
          {applyLabel}
        </Button>
      </Space>
      {result ? (
        <div style={{ marginTop: 4, padding: 8, background: "#fafafa", borderRadius: 4 }}>
          {result.ok ? (
            isMaskRow ? (
              <Space size="small" wrap align="center">
                <Text type="secondary" style={{ fontSize: 12 }}>脱敏结果：</Text>
                <Text strong copyable>{result.masked}</Text>
              </Space>
            ) : (
              <Space size="small" wrap align="center">
                <Text type="secondary" style={{ fontSize: 12 }}>{resultLabel}</Text>
                {result.valid ? (
                  <Text type="success" strong>{validText}</Text>
                ) : (
                  <Text type="danger" strong>{validText}</Text>
                )}
                {result.message ? (
                  <Text type="secondary" style={{ fontSize: 12 }}>（{result.message}）</Text>
                ) : null}
              </Space>
            )
          ) : (
            <Text type="danger" style={{ fontSize: 12 }}>错误：{result.error}</Text>
          )}
        </div>
      ) : null}
    </div>
  );
}

// 规则管理视图（v0.5.x Table 化）：
// - 启动时 state.rules 来自 builtin_ruleset()：3 条数据提取规则 + 4 条
//   通用脱敏模版 + 4 条数据校验规则。
// - 用 antd Table 替代竖向 Card 列表；规则行默认全部收起（expandedRowKeys
//   受控，初始化为空数组），点击行展开按钮才展开参数表单。
// - 保留 tag 过滤 Select + 「展开/收起参数」切换按钮。
// - mask 规则行内含参数表单 + 样例值 + 运行 + 应用并跳转；其它规则展开行显示描述全文。
export default function RulesView({ state, dispatch }) {
  const { rules, rulesTagFilter } = state;

  // 可选标签列表：来自 list_rule_tags 命令（静态三选一）。
  // 失败退化为静态三选一，保证过滤 Select 始终可用。
  const [tagOptions, setTagOptions] = React.useState([
    "extract",
    "mask",
    "validate",
  ]);
  const rulesJson = React.useMemo(() => {
    return JSON.stringify({
      maskers: rules.maskers,
      validators: rules.validators,
    });
  }, [rules]);
  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const tags = await listRuleTags(rulesJson);
        if (cancelled) return;
        setTagOptions(Array.isArray(tags) && tags.length ? tags : ["extract", "mask", "validate"]);
      } catch {
        if (!cancelled) setTagOptions(["extract", "mask", "validate"]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [rulesJson]);

  // 合并 maskers + validators 为单一列表，每条带 __kind / __index / key。
  const merged = React.useMemo(() => {
    const maskers = rules.maskers.map((r, i) => ({ ...r, __kind: "mask", __index: i, key: `mask-${i}` }));
    const validators = rules.validators.map((r, i) => ({ ...r, __kind: "validate", __index: i, key: `validate-${i}` }));
    return [...maskers, ...validators];
  }, [rules]);

  // mask 行 key（保留，用于「全部展开/收起」按钮）。
  const maskKeys = React.useMemo(
    () => merged.filter((r) => r.__kind === "mask" && r.tag === "mask").map((r) => r.key),
    [merged]
  );

  // 默认全部收起，点击展开按钮才展开。
  const [expandedKeys, setExpandedKeys] = React.useState([]);
  const maskKeysSig = maskKeys.join(",");
  React.useEffect(() => {
    // maskers 变化时重置为全部收起。
    setExpandedKeys([]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [maskKeysSig]);

  // 行内表单状态：{ [key]: { params, sample, field, result, loading, applying } }
  const [rowStates, setRowStates] = React.useState({});
  const setRowState = React.useCallback((key, updater) => {
    setRowStates((prev) => {
      const prevRow = prev[key] ?? {};
      const nextRow = typeof updater === "function" ? updater(prevRow) : updater;
      return { ...prev, [key]: { ...prevRow, ...nextRow } };
    });
  }, []);

  // 按标签过滤：rulesTagFilter 为 null 时显示全部，否则只显示 tag == rulesTagFilter 的规则。
  const filtered = React.useMemo(
    () => (rulesTagFilter ? merged.filter((r) => r.tag === rulesTagFilter) : merged),
    [merged, rulesTagFilter]
  );
  // 过滤后可见的 mask key（用于全部展开/收起）。
  const filteredMaskKeys = React.useMemo(
    () => filtered.filter((r) => r.__kind === "mask" && r.tag === "mask").map((r) => r.key),
    [filtered]
  );

  const allExpanded = filteredMaskKeys.length > 0 && filteredMaskKeys.every((k) => expandedKeys.includes(k));
  function toggleAll() {
    if (allExpanded) {
      setExpandedKeys((prev) => prev.filter((k) => !filteredMaskKeys.includes(k)));
    } else {
      setExpandedKeys((prev) => Array.from(new Set([...prev, ...filteredMaskKeys])));
    }
  }

  // 试运行：操作列按钮与展开区共享同一逻辑。
  async function runTrialForRow(record) {
    const rs = rowStates[record.key] ?? {};
    if (!rs.sample) {
      message.warning("请输入样例值再试运行");
      setExpandedKeys((prev) => Array.from(new Set([...prev, record.key])));
      return;
    }
    const paramsJson = JSON.stringify(buildParams(record.scope, rs.params ?? {}));
    setRowState(record.key, (s) => ({ ...s, loading: true, result: null }));
    try {
      const r = await trialMask(record.scope, paramsJson, rs.sample);
      setRowState(record.key, (s) => ({ ...s, loading: false, result: r }));
      if (r?.ok) {
        message.success("试运行完成");
      } else {
        message.error(r?.error || "试运行失败");
      }
    } catch (e) {
      const err = typeof e === "string" ? e : String(e);
      setRowState(record.key, (s) => ({ ...s, loading: false, result: { ok: false, masked: null, error: err } }));
      message.error(err);
    }
  }

  // 应用并跳转：把 scope + params 写为 maskOverrides 中一条动态规则。
  async function applyForRow(record) {
    const rs = rowStates[record.key] ?? {};
    setRowState(record.key, (s) => ({ ...s, applying: true }));
    try {
      const targetField = (rs.field || "").trim() || record.field;
      const paramsObj = buildParams(record.scope, rs.params ?? {});
      const ruleObj = {
        field: targetField,
        scope: record.scope,
        tag: "mask",
        params: Object.keys(paramsObj).length ? paramsObj : undefined,
        message: undefined,
        description: undefined,
      };
      dispatch({ type: "SET_MASK_OVERRIDE", header: targetField, rule: ruleObj });
      const headers = state?.records?.headers || [];
      if (headers.length > 0) {
        if (headers.includes(targetField)) {
          message.success(`已应用「${targetField}」列脱敏配置，跳转数据脱敏`);
        } else {
          message.warning(`字段「${targetField}」未在当前表头中找到，已写入配置，请在数据脱敏视图调整字段名`);
        }
      } else {
        message.success("已应用脱敏配置，请先到数据预处理导入文件后再到数据脱敏视图使用");
      }
      dispatch({ type: "SET_VIEW", activeView: "mask" });
    } finally {
      setRowState(record.key, (s) => ({ ...s, applying: false }));
    }
  }

  // 校验/提取试运行（v0.6.2，v0.6.8 扩展）：与 runTrialForRow 对称，调 trial_validate。
  // v0.6.8（修订）：phone（extract）与 pinfo_phone（validate）scope 已在 trial_validate
  // 后端命令中支持，复用同一试运行入口。
  async function runValidateTrialForRow(record) {
    const rs = rowStates[record.key] ?? {};
    if (!rs.sample) {
      message.warning("请输入样例值再试运行");
      setExpandedKeys((prev) => Array.from(new Set([...prev, record.key])));
      return;
    }
    const paramsJson = JSON.stringify(buildParams(record.scope, rs.params ?? {}));
    setRowState(record.key, (s) => ({ ...s, loading: true, result: null }));
    try {
      const r = await trialValidate(record.scope, paramsJson, rs.sample);
      setRowState(record.key, (s) => ({ ...s, loading: false, result: r }));
      if (r?.ok) {
        message.success("试运行完成");
      } else {
        message.error(r?.error || "试运行失败");
      }
    } catch (e) {
      const err = typeof e === "string" ? e : String(e);
      setRowState(record.key, (s) => ({ ...s, loading: false, result: { ok: false, valid: null, error: err } }));
      message.error(err);
    }
  }

  // 校验应用并跳转（v0.6.2）：把 scope + params 写为 validateOverrides 中一条动态规则。
  async function applyValidateForRow(record) {
    const rs = rowStates[record.key] ?? {};
    setRowState(record.key, (s) => ({ ...s, applying: true }));
    try {
      const targetField = (rs.field || "").trim() || record.field;
      const paramsObj = buildParams(record.scope, rs.params ?? {});
      const ruleObj = {
        field: targetField,
        scope: record.scope,
        tag: "validate",
        params: Object.keys(paramsObj).length ? paramsObj : undefined,
        message: undefined,
        description: undefined,
      };
      dispatch({ type: "SET_VALIDATE_OVERRIDE", header: targetField, rule: ruleObj });
      const headers = state?.records?.headers || [];
      if (headers.length > 0) {
        if (headers.includes(targetField)) {
          message.success(`已应用「${targetField}」列校验配置，跳转数据校验`);
        } else {
          message.warning(`字段「${targetField}」未在当前表头中找到，已写入配置，请在数据校验视图调整字段名`);
        }
      } else {
        message.success("已应用校验配置，请先到数据预处理导入文件后再到数据校验视图使用");
      }
      dispatch({ type: "SET_VIEW", activeView: "validate" });
    } finally {
      setRowState(record.key, (s) => ({ ...s, applying: false }));
    }
  }

  // 提取应用并跳转（v0.6.8 修订）：把 scope + params 写为 extractOverrides 中一条动态规则。
  // 与 applyValidateForRow 对称，但 tag="extract"、写入 extractOverrides、跳数据提取视图。
  async function applyExtractForRow(record) {
    const rs = rowStates[record.key] ?? {};
    setRowState(record.key, (s) => ({ ...s, applying: true }));
    try {
      const targetField = (rs.field || "").trim() || record.field;
      const paramsObj = buildParams(record.scope, rs.params ?? {});
      const ruleObj = {
        field: targetField,
        scope: record.scope,
        tag: "extract",
        params: Object.keys(paramsObj).length ? paramsObj : undefined,
        message: undefined,
        description: undefined,
      };
      dispatch({ type: "SET_EXTRACT_OVERRIDE", header: targetField, rule: ruleObj });
      const headers = state?.records?.headers || [];
      if (headers.length > 0) {
        if (headers.includes(targetField)) {
          message.success(`已应用「${targetField}」提取配置，跳转数据提取`);
        } else {
          message.warning(`字段「${targetField}」未在当前表头中找到，已写入配置，请在数据提取视图调整字段名`);
        }
      } else {
        message.success("已应用提取配置，请先到数据预处理导入文件后再到数据提取视图使用");
      }
      dispatch({ type: "SET_VIEW", activeView: "extract" });
    } finally {
      setRowState(record.key, (s) => ({ ...s, applying: false }));
    }
  }

  const columns = [
    {
      title: "标签",
      dataIndex: "tag",
      width: 100,
      render: (tag) => <Tag color={tagColor(tag)} style={{ margin: 0 }}>{tagLabel(tag)}</Tag>,
    },
    {
      title: "字段",
      dataIndex: "field",
      width: 140,
      render: (t) => <Text strong>{t || "（未指定字段）"}</Text>,
    },
    {
      title: "算子",
      dataIndex: "scope",
      width: 150,
      render: (s) => <Tag style={{ margin: 0 }}>{s}</Tag>,
    },
    {
      title: "描述",
      dataIndex: "description",
      ellipsis: true,
      render: (d) => d ? <Text type="secondary" style={{ fontSize: 12 }}>{d}</Text> : null,
    },
    {
      title: "操作",
      key: "action",
      width: 170,
      render: (_, r) => {
        // 有参数元信息的规则（mask 模版 + regex/pinfo_phone 校验 + phone 提取）才显示运行/应用按钮。
        if (getParamMeta(r.scope).length === 0) return null;
        const isMaskRow = r.__kind === "mask" && r.tag === "mask";
        const isExtractRow = r.__kind === "validate" && r.tag === "extract";
        const rs = rowStates[r.key] ?? {};
        const runHandler = isMaskRow ? runTrialForRow : isExtractRow ? runValidateTrialForRow : runValidateTrialForRow;
        const applyHandler = isMaskRow ? applyForRow : isExtractRow ? applyExtractForRow : applyValidateForRow;
        return (
          <Space size="small">
            <Button
              size="small"
              type="primary"
              icon={<ExperimentOutlined />}
              loading={!!rs.loading}
              onClick={() => runHandler(r)}
            >
              运行
            </Button>
            <Button
              size="small"
              icon={<RocketOutlined />}
              loading={!!rs.applying}
              onClick={() => applyHandler(r)}
            >
              应用
            </Button>
          </Space>
        );
      },
    },
  ];

  return (
    <Card title="规则管理" styles={{ body: { padding: 12 } }}>
      {merged.length === 0 ? (
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无内置规则" />
      ) : (
        <>
          <div style={{ marginBottom: 8, display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              共 {rules.maskers.length + rules.validators.length} 条规则
              （脱敏 {rules.maskers.length} · 提取/校验 {rules.validators.length}）
            </Text>
            <Select
              size="small"
              style={{ width: 180 }}
              placeholder="按标签过滤"
              value={rulesTagFilter ?? "__all__"}
              onChange={(v) => {
                dispatch({
                  type: "SET_RULES_TAG_FILTER",
                  tag: v === "__all__" ? null : v,
                });
              }}
              options={[
                { label: "全部", value: "__all__" },
                ...tagOptions.map((t) => ({ label: `${tagLabel(t)} (${t})`, value: t })),
              ]}
              allowClear
              onClear={() => dispatch({ type: "SET_RULES_TAG_FILTER", tag: null })}
            />
            <Button size="small" onClick={toggleAll}>
              {allExpanded ? "收起参数" : "展开参数"}
            </Button>
            {rulesTagFilter ? (
              <Text type="secondary" style={{ fontSize: 12 }}>
                当前过滤：{tagLabel(rulesTagFilter)}（命中 {filtered.length} 条）
              </Text>
            ) : null}
          </div>
          {filtered.length === 0 ? (
            <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={`无「${tagLabel(rulesTagFilter)}」标签的规则`} />
          ) : (
            <Table
              size="small"
              pagination={false}
              scroll={{ y: 480, x: "max-content" }}
              sticky
              columns={columns}
              dataSource={filtered}
              rowKey="key"
              expandable={{
                expandedRowKeys: expandedKeys,
                onExpandedRowsChange: (keys) => setExpandedKeys(keys),
                rowExpandable: () => true,
                expandedRowRender: (record) => (
                  <RowExpanded
                    record={record}
                    rowState={rowStates[record.key] ?? {}}
                    setRowState={setRowState}
                    state={state}
                    dispatch={dispatch}
                    onRun={record.__kind === "mask"
                      ? runTrialForRow
                      : (record.tag === "extract" ? runValidateTrialForRow : runValidateTrialForRow)}
                    onApply={record.__kind === "mask"
                      ? applyForRow
                      : (record.tag === "extract" ? applyExtractForRow : applyValidateForRow)}
                  />
                ),
              }}
            />
          )}
          <Text type="secondary" style={{ display: "block", marginTop: 8, fontSize: 12 }}>
            脱敏模版参数为空时使用后端默认值；试运行仅用于验证参数，不会改动真实数据。
            试运行通过后点「应用」可把当前参数写入数据脱敏视图的表头映射并跳转。
            校验模版填写 pattern 后试运行验证正则，应用后写入数据校验视图。
          </Text>
        </>
      )}
    </Card>
  );
}
