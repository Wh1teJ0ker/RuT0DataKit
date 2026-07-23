import React from "react";
import { Card, Empty, Select, Typography, Tag, Space } from "antd";
import { listRuleTags } from "../tauri.js";

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

// 规则管理视图（v0.4.4 规则引擎重构后）：
// - 规则池初始为空，无添加规则入口（后续版本接入）。
// - 保留 tag 过滤 Select（静态三选一：数据提取 / 数据脱敏 / 数据校验）。
// - 列表展示每条规则的 field / scope / tag / description。
// - 空态文案「暂无规则，后续版本将提供规则添加入口」。
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

  // 合并 maskers + validators 为单一列表，每条带 __kind / __index。
  const merged = [
    ...rules.maskers.map((r, i) => ({ ...r, __kind: "mask", __index: i })),
    ...rules.validators.map((r, i) => ({ ...r, __kind: "validate", __index: i })),
  ];
  // 按标签过滤：rulesTagFilter 为 null 时显示全部，否则只显示 tag == rulesTagFilter 的规则。
  // v0.4.4：tag 为单值字段（非多标签数组），改为严格相等匹配。
  const filtered = rulesTagFilter
    ? merged.filter((r) => r.tag === rulesTagFilter)
    : merged;

  return (
    <Card
      title="规则管理"
      styles={{ body: { padding: 12 } }}
    >
      {merged.length === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="暂无规则，后续版本将提供规则添加入口"
        />
      ) : (
        <>
          <div style={{ marginBottom: 8, display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              共 {rules.maskers.length + rules.validators.length} 条规则
              （脱敏 {rules.maskers.length} · 校验 {rules.validators.length}）
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
              onClear={() =>
                dispatch({ type: "SET_RULES_TAG_FILTER", tag: null })
              }
            />
            {rulesTagFilter ? (
              <Text type="secondary" style={{ fontSize: 12 }}>
                当前过滤：{tagLabel(rulesTagFilter)}（命中 {filtered.length} 条）
              </Text>
            ) : null}
          </div>
          {filtered.length === 0 ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={`无「${tagLabel(rulesTagFilter)}」标签的规则`}
            />
          ) : (
            <Space direction="vertical" size="small" style={{ width: "100%" }}>
              {filtered.map((item) => (
                <Card
                  key={`${item.__kind}-${item.__index}`}
                  size="small"
                  styles={{ body: { padding: "8px 12px" } }}
                >
                  <Space size="small" wrap align="center">
                    <Tag color={tagColor(item.tag)} style={{ margin: 0 }}>
                      {tagLabel(item.tag)}
                    </Tag>
                    <Text strong>{item.field || "（未指定字段）"}</Text>
                    <Text type="secondary">→</Text>
                    <Tag style={{ margin: 0 }}>{item.scope}</Tag>
                    {item.description ? (
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {item.description}
                      </Text>
                    ) : null}
                  </Space>
                </Card>
              ))}
            </Space>
          )}
        </>
      )}
    </Card>
  );
}
