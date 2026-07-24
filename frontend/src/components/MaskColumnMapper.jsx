import React, { useMemo } from "react";
import {
  Card,
  Table,
  Select,
  Input,
  InputNumber,
  Switch,
  Button,
  Typography,
  Space,
  Empty,
  Tooltip,
  Alert,
} from "antd";
import { DeleteOutlined } from "@ant-design/icons";

const { Text } = Typography;

// 脱敏算子（scope）清单：仅脱敏侧，固定 4 种 MaskOp 通用算子。
// tag 统一固定 "mask"，field 随表头动态填，参数由用户按列现场填。
const SCOPE_OPTIONS = [
  { label: "替换模版 (template)", value: "template" },
  { label: "切分模版 (split_template)", value: "split_template" },
  { label: "正则替换 (regex_replace)", value: "regex_replace" },
  { label: "常量替换 (const_replace)", value: "const_replace" },
];

// 把一条 override 规则摘要成一行文本，便于在表格里快速识别当前配置。
function summarizeRule(r) {
  if (!r) return "不脱敏";
  const paramsStr = r.params
    ? Object.entries(r.params)
        .map(([k, v]) => `${k}=${v == null ? "" : String(v)}`)
        .join(", ")
    : "";
  return `${r.scope || ""}${paramsStr ? ` (${paramsStr})` : ""}`;
}

// 表头-脱敏映射组件（段②）。
// 与 ValidateView 的 ColumnRuleMapper 不同：脱敏规则不来自「规则管理」全局库，
// 而是每列现场选 scope + 填参数的「动态规则」。用户为每个表头选算子并填参数，
// 直接写 maskOverrides（会话级，不污染全局规则库，导入新文件时清空）。
// 无规则时该列不脱敏。
//
// props:
//   headers        表头列表
//   overrides      state.maskOverrides { [header]: MaskRule }
//   hasRecords     是否有原始数据
//   onSetOverride  (header, rule) => void
//   onClearOverride (header) => void
export default function MaskColumnMapper({
  headers,
  overrides,
  hasRecords,
  onSetOverride,
  onClearOverride,
}) {
  const overridesMap = overrides || {};

  const mappingData = useMemo(
    () => headers.map((h, idx) => ({ key: idx, header: h })),
    [headers]
  );

  // 取某表头当前 override（可能为 undefined = 不脱敏）。
  const resolveRule = (header) => overridesMap[header];

  // 修改某表头的 scope：保留已有 params 中两 scope 共有的字段，其余重置。
  const onChangeScope = (header, scope) => {
    const prev = overridesMap[header];
    const prevParams = prev?.params || {};
    const nextParams = {};
    // template / split_template 共享 7 个参数
    const sharedTpl = ["keep_prefix", "keep_suffix", "mask_char", "mask_min_len", "min_len", "max_len", "cjk"];
    if (scope === "template" || scope === "split_template") {
      sharedTpl.forEach((k) => {
        if (prevParams[k] !== undefined) nextParams[k] = prevParams[k];
      });
    }
    if (scope === "split_template" && prev?.scope === "split_template") {
      if (prevParams.separator !== undefined) nextParams.separator = prevParams.separator;
      if (prevParams.segment_index !== undefined) nextParams.segment_index = prevParams.segment_index;
    }
    // 切换到非模版算子时清空模版参数
    onSetOverride(header, {
      field: header,
      scope,
      tag: "mask",
      params: Object.keys(nextParams).length ? nextParams : undefined,
      message: undefined,
      description: undefined,
    });
  };

  // 修改某表头的单个参数。
  const onParamChange = (header, key, value) => {
    const prev = overridesMap[header];
    if (!prev) return;
    const nextParams = { ...(prev.params || {}) };
    if (value === undefined || value === null || value === "") {
      delete nextParams[key];
    } else {
      nextParams[key] = value;
    }
    onSetOverride(header, { ...prev, params: Object.keys(nextParams).length ? nextParams : undefined });
  };

  const mappingColumns = [
    {
      title: "表头",
      dataIndex: "header",
      width: 140,
      render: (text, row) => {
        const has = !!resolveRule(row.header);
        return (
          <Space size={4}>
            <span
              style={{
                display: "inline-block",
                width: 6,
                height: 6,
                borderRadius: "50%",
                background: has ? "#1677ff" : "#d9d9d9",
              }}
            />
            <Text strong>{text}</Text>
          </Space>
        );
      },
    },
    {
      title: "脱敏算子 (scope)",
      dataIndex: "header",
      key: "scope",
      width: 200,
      render: (_, row) => {
        const rule = resolveRule(row.header);
        return (
          <Select
            value={rule?.scope}
            style={{ width: "100%" }}
            placeholder="不脱敏"
            options={SCOPE_OPTIONS}
            allowClear
            onChange={(v) => {
              if (v == null) {
                onClearOverride(row.header);
              } else {
                onChangeScope(row.header, v);
              }
            }}
          />
        );
      },
    },
    {
      title: "参数",
      key: "params",
      render: (_, row) => {
        const rule = resolveRule(row.header);
        if (!rule) {
          return (
            <Text type="secondary" style={{ fontSize: 12 }}>
              不脱敏
            </Text>
          );
        }
        return (
          <ParamsForm rule={rule} onParamChange={(k, v) => onParamChange(row.header, k, v)} />
        );
      },
    },
    {
      title: "摘要",
      key: "summary",
      width: 200,
      render: (_, row) => {
        const existing = resolveRule(row.header);
        return (
          <Text type="secondary" style={{ fontSize: 12 }}>
            {summarizeRule(existing)}
          </Text>
        );
      },
    },
    {
      title: "操作",
      key: "actions",
      width: 60,
      align: "center",
      render: (_, row) => {
        const existing = resolveRule(row.header);
        if (!existing) {
          return <Text type="secondary" style={{ fontSize: 12 }}>—</Text>;
        }
        return (
          <Tooltip title="清除该列脱敏配置">
            <Button
              type="text"
              danger
              size="small"
              icon={<DeleteOutlined />}
              onClick={() => onClearOverride(row.header)}
            />
          </Tooltip>
        );
      },
    },
  ];

  return (
    <Card
      title="表头-脱敏映射"
      styles={{ body: { padding: 12 } }}
      extra={
        <Text type="secondary" style={{ fontSize: 12 }}>
          为每个表头选脱敏算子并填参数；未选则该列不脱敏。参数随列现场配置，不进规则库。
        </Text>
      }
    >
      {hasRecords ? (
        <Table
          size="small"
          pagination={false}
          scroll={{ y: 280, x: "max-content" }}
          locale={{ emptyText: "导入文件后此处显示表头" }}
          columns={mappingColumns}
          dataSource={mappingData}
        />
      ) : (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="请先到数据预处理导入文件"
        />
      )}
    </Card>
  );
}

// 按算子类型渲染的参数表单。所有参数空值不写入 params（让后端默认生效）。
function ParamsForm({ rule, onParamChange }) {
  const scope = rule?.scope;
  if (!scope) return null;
  const p = rule.params || {};

  if (scope === "template" || scope === "split_template") {
    return (
      <Space size="small" wrap align="center">
        {scope === "split_template" ? (
          <>
            <ParamItem label="separator">
              <Input
                size="small"
                style={{ width: 70 }}
                placeholder="@"
                value={p.separator ?? ""}
                onChange={(e) => onParamChange("separator", e.target.value || undefined)}
              />
            </ParamItem>
            <ParamItem label="segment">
              <InputNumber
                size="small"
                style={{ width: 70 }}
                min={0}
                placeholder="0"
                value={p.segment_index}
                onChange={(v) => onParamChange("segment_index", v ?? undefined)}
              />
            </ParamItem>
          </>
        ) : null}
        <ParamItem label="前缀">
          <InputNumber
            size="small"
            style={{ width: 64 }}
            min={0}
            placeholder="0"
            value={p.keep_prefix}
            onChange={(v) => onParamChange("keep_prefix", v ?? undefined)}
          />
        </ParamItem>
        <ParamItem label="后缀">
          <InputNumber
            size="small"
            style={{ width: 64 }}
            min={0}
            placeholder="0"
            value={p.keep_suffix}
            onChange={(v) => onParamChange("keep_suffix", v ?? undefined)}
          />
        </ParamItem>
        <ParamItem label="最少脱敏">
          <InputNumber
            size="small"
            style={{ width: 70 }}
            min={0}
            placeholder="1"
            value={p.mask_min_len}
            onChange={(v) => onParamChange("mask_min_len", v ?? undefined)}
          />
        </ParamItem>
        <ParamItem label="下限">
          <InputNumber
            size="small"
            style={{ width: 64 }}
            min={0}
            placeholder="可空"
            value={p.min_len}
            onChange={(v) => onParamChange("min_len", v ?? undefined)}
          />
        </ParamItem>
        <ParamItem label="上限">
          <InputNumber
            size="small"
            style={{ width: 64 }}
            min={0}
            placeholder="可空"
            value={p.max_len}
            onChange={(v) => onParamChange("max_len", v ?? undefined)}
          />
        </ParamItem>
        <ParamItem label="字符">
          <Input
            size="small"
            style={{ width: 54 }}
            maxLength={1}
            placeholder="*"
            value={p.mask_char ?? ""}
            onChange={(e) => onParamChange("mask_char", e.target.value || undefined)}
          />
        </ParamItem>
        <ParamItem label="cjk">
          <Switch
            size="small"
            checked={!!p.cjk}
            onChange={(v) => onParamChange("cjk", v)}
          />
        </ParamItem>
      </Space>
    );
  }

  if (scope === "regex_replace") {
    return (
      <Space size="small" wrap align="center" style={{ width: "100%" }}>
        <ParamItem label="pattern">
          <Input
            size="small"
            style={{ width: 160 }}
            placeholder="正则"
            value={p.pattern ?? ""}
            onChange={(e) => onParamChange("pattern", e.target.value || undefined)}
          />
        </ParamItem>
        <ParamItem label="replacement">
          <Input
            size="small"
            style={{ width: 120 }}
            placeholder="替换文本"
            value={p.replacement ?? ""}
            onChange={(e) => onParamChange("replacement", e.target.value || undefined)}
          />
        </ParamItem>
        <ParamItem label="模式">
          <Select
            size="small"
            style={{ width: 90 }}
            value={p.match_mode ?? "all"}
            options={[
              { label: "全部", value: "all" },
              { label: "首个", value: "first" },
            ]}
            onChange={(v) => onParamChange("match_mode", v)}
          />
        </ParamItem>
      </Space>
    );
  }

  if (scope === "const_replace") {
    return (
      <ParamItem label="替换为">
        <Input
          size="small"
          style={{ width: 160 }}
          placeholder="如 ***"
          value={p.with ?? ""}
          onChange={(e) => onParamChange("with", e.target.value || undefined)}
        />
      </ParamItem>
    );
  }

  return null;
}

function ParamItem({ label, children }) {
  return (
    <Space size={4} align="center">
      <Text type="secondary" style={{ fontSize: 11 }}>
        {label}
      </Text>
      {children}
    </Space>
  );
}
