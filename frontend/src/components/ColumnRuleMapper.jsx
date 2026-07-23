import React, { useMemo } from "react";
import {
  Card,
  Table,
  Select,
  Button,
  Typography,
  Space,
  Empty,
  Tooltip,
  Alert,
} from "antd";
import { DeleteOutlined } from "@ant-design/icons";

const { Text } = Typography;

// 把一条规则摘要成下拉项 label：`scope → field(params)`，便于用户在下拉里识别。
// MaskView / ValidateView 原先各自重复实现，v0.5.0 T12-6 抽到公共组件。
function summarizeRule(r) {
  const paramsStr = r.params
    ? Object.entries(r.params)
        .map(([k, v]) => `${k}=${v == null ? "" : String(v)}`)
        .join(", ")
    : "";
  return `${r.scope || "（未指定 scope）"} → ${r.field || "（未指定字段）"}${paramsStr ? ` (${paramsStr})` : ""}`;
}

// 表头-规则映射公共组件（段②）。
// 封装 MaskView / ValidateView 高度重复的「表头-规则映射 Table + 规则下拉 +
// 摘要列 + 清空按钮」结构。组件只负责展示与回调，不直接 dispatch——调用方在
// onSetOverride / onClearOverride / onGotoRules 回调里 dispatch，保持纯展示。
//
// rules 数组由调用方负责按 tag 筛选后传入（更解耦，组件内不再按 tag 筛选）。
// sentinel 为「不应用」选项哨兵值（如 "__no_mask__" / "__no_validate__"），
// 组件内不硬编码，由调用方通过 props 传入。
//
// props:
//   headers         表头列表
//   rules           已筛选的规则数组（调用方负责按 tag 筛）
//   overrides       当前映射 { [header]: rule }
//   sentinel        「不应用」选项哨兵值
//   sentinelLabel   哨兵项显示文本（如「不脱敏」/「不校验」）
//   ruleColumnTitle 规则列标题（如「脱敏规则」/「校验规则」）
//   noRuleTitle     无规则时 Alert 的 message（如「暂无脱敏规则」）
//   noRuleDesc      无规则时 Alert 的 description
//   hint            Card extra 提示文本
//   hasRecords      是否有原始数据（false 时渲染 Empty 引导）
//   onSetOverride   (header, rule) => void
//   onClearOverride (header) => void
//   onGotoRules     () => void（跳转规则管理视图）
export default function ColumnRuleMapper({
  headers,
  rules,
  overrides,
  sentinel,
  sentinelLabel,
  ruleColumnTitle,
  noRuleTitle,
  noRuleDesc,
  hint,
  hasRecords,
  onSetOverride,
  onClearOverride,
  onGotoRules,
}) {
  const rulesArr = rules || [];
  const overridesMap = overrides || {};

  const mappingData = useMemo(
    () => headers.map((h, idx) => ({ key: idx, header: h })),
    [headers]
  );

  // 下拉源：哨兵「不应用」+ 用户创建的规则。按规则在 ruleset 的下标作 value。
  const ruleOptions = useMemo(() => {
    const opts = rulesArr.map((r, i) => ({
      value: i,
      label: summarizeRule(r),
    }));
    return [{ value: sentinel, label: sentinelLabel }, ...opts];
  }, [rulesArr, sentinel, sentinelLabel]);

  // 取某表头当前生效的规则（override 优先，rules 兜底）。返回 null 表示不应用。
  const resolveRule = (header) => {
    if (overridesMap[header]) return overridesMap[header];
    return rulesArr.find((r) => r.field === header) || null;
  };

  // 根据当前生效规则定位下拉 value：
  // - 无规则 → sentinel
  // - override / 兜底命中 → 该规则在 rulesArr 中的下标（按引用 + 字段对比）
  const resolveOptionValue = (header) => {
    const existing = resolveRule(header);
    if (!existing) return sentinel;
    const idx = rulesArr.findIndex(
      (r) =>
        r === existing ||
        (r.field === existing.field &&
          r.scope === existing.scope &&
          JSON.stringify(r.params || {}) ===
            JSON.stringify(existing.params || {}))
    );
    return idx >= 0 ? idx : sentinel;
  };

  const onChangeRule = (header, selectedValue) => {
    if (selectedValue === sentinel) {
      onClearOverride(header);
      return;
    }
    const rule = rulesArr[selectedValue];
    if (!rule) return;
    // 把规则原样作为 override 写入（保留 masker/validator + params + description）。
    onSetOverride(header, { ...rule });
  };

  const mappingColumns = [
    {
      title: "表头",
      dataIndex: "header",
      width: 160,
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
      title: ruleColumnTitle,
      dataIndex: "header",
      key: "rule",
      render: (_, row) => {
        const value = resolveOptionValue(row.header);
        return (
          <Select
            value={value}
            showSearch
            style={{ width: "100%" }}
            options={ruleOptions}
            placeholder={
              rulesArr.length === 0
                ? "暂无规则，请先到「规则管理」创建"
                : `选择${ruleColumnTitle}`
            }
            onChange={(v) => onChangeRule(row.header, v)}
          />
        );
      },
    },
    {
      title: "规则摘要",
      key: "summary",
      render: (_, row) => {
        const existing = resolveRule(row.header);
        if (!existing) {
          return (
            <Text type="secondary" style={{ fontSize: 12 }}>
              {sentinelLabel}
            </Text>
          );
        }
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
      width: 80,
      align: "center",
      render: (_, row) => {
        const existing = resolveRule(row.header);
        if (!existing) {
          return (
            <Text type="secondary" style={{ fontSize: 12 }}>
              —
            </Text>
          );
        }
        return (
          <Tooltip title="清除该列映射（不影响规则库）">
            <span>
              <Button
                type="text"
                danger
                size="small"
                icon={<DeleteOutlined />}
                onClick={() => onClearOverride(row.header)}
              />
            </span>
          </Tooltip>
        );
      },
    },
  ];

  return (
    <Card
      title="表头-规则映射"
      styles={{ body: { padding: 12 } }}
      extra={
        <Text type="secondary" style={{ fontSize: 12 }}>
          {hint}
        </Text>
      }
    >
      {hasRecords ? (
        rulesArr.length === 0 ? (
          <Alert
            type="info"
            showIcon
            message={noRuleTitle}
            description={noRuleDesc}
            action={
              <Button size="small" type="primary" onClick={onGotoRules}>
                去创建
              </Button>
            }
          />
        ) : (
          <Table
            size="small"
            pagination={false}
            scroll={{ y: 240 }}
            locale={{ emptyText: "导入文件后此处显示表头" }}
            columns={mappingColumns}
            dataSource={mappingData}
          />
        )
      ) : (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="请先到数据预处理导入文件"
        />
      )}
    </Card>
  );
}
