import React from "react";
import {
  Card,
  Space,
  Button,
  List,
  Tag,
  Empty,
  Tooltip,
  Typography,
  Spin,
  Alert,
  Input,
  Select,
  App as AntApp,
} from "antd";
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  PlayCircleOutlined,
  ArrowRightOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
} from "@ant-design/icons";
import RuleDrawer from "./RuleDrawer.jsx";
import { previewMaskRuleValue, previewValidateRuleValue, listRuleTags } from "../tauri.js";

const { Text, Paragraph } = Typography;

// antd Tag 预置色板（与 antd 文档一致）：tag 颜色按字符串哈希取模映射，
// 同一 tag 始终同色。
const TAG_PRESET_COLORS = [
  "magenta",
  "red",
  "volcano",
  "orange",
  "gold",
  "lime",
  "green",
  "cyan",
  "blue",
  "geekblue",
  "purple",
];

function tagColor(tag) {
  let h = 0;
  for (let i = 0; i < tag.length; i++) {
    h = (h * 31 + tag.charCodeAt(i)) >>> 0;
  }
  return TAG_PRESET_COLORS[h % TAG_PRESET_COLORS.length];
}

// 规则管理视图（独立系统）：不依赖任何数据文件导入。
// - 右上角仅保留「添加规则」按钮（不再有保存为 YAML / 从 YAML 加载）。
// - 单一列表合并脱敏 + 校验规则，每条卡片带「试运行」按钮。
// - 试运行改为用户在卡片内手动输入一个样例值，直接对算子跑一次，
//   完全不读取已导入数据文件，规则管理与数据文件彻底解耦。
export default function RulesView({ state, dispatch }) {
  const { rules, rulesTagFilter } = state;

  // 可选标签列表：来自 list_rule_tags 命令（预置标签 ∪ 当前 ruleset 出现的 tag）。
  // Drawer 加载/编辑时也会拉，但这里只用于顶部过滤 Select；失败退化为空数组。
  const [tagOptions, setTagOptions] = React.useState([]);
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
        setTagOptions(Array.isArray(tags) ? tags : []);
      } catch {
        if (!cancelled) setTagOptions([]);
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
  // 按标签过滤：rulesTagFilter 为 null 时显示全部，否则只显示 tags 含该 tag 的规则。
  const filtered = rulesTagFilter
    ? merged.filter((r) => Array.isArray(r.tags) && r.tags.includes(rulesTagFilter))
    : merged;

  const handleAdd = () => {
    dispatch({ type: "SET_EDITING_RULE", rule: { kind: "mask" } });
  };

  const handleEdit = (item) => {
    dispatch({
      type: "SET_EDITING_RULE",
      rule: { ...item, kind: item.__kind, __index: item.__index },
    });
  };

  const handleRemove = (item) => {
    dispatch({ type: "REMOVE_RULE", kind: item.__kind, index: item.__index });
  };

  return (
    <Card
      title="规则管理"
      styles={{ body: { padding: 12 } }}
      extra={
        <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>
          添加规则
        </Button>
      }
    >
      {merged.length === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="暂无规则，点击右上角「添加规则」创建"
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
                ...tagOptions.map((t) => ({ label: t, value: t })),
              ]}
              allowClear
              onClear={() =>
                dispatch({ type: "SET_RULES_TAG_FILTER", tag: null })
              }
            />
            {rulesTagFilter ? (
              <Text type="secondary" style={{ fontSize: 12 }}>
                当前过滤：{rulesTagFilter}（命中 {filtered.length} 条）
              </Text>
            ) : null}
          </div>
          {filtered.length === 0 ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={`无带「${rulesTagFilter}」标签的规则`}
            />
          ) : (
            <List
              dataSource={filtered}
              renderItem={(item) => (
                <RuleCard
                  key={`${item.__kind}-${item.__index}`}
                  item={item}
                  onEdit={handleEdit}
                  onRemove={handleRemove}
                />
              )}
            />
          )}
        </>
      )}
      <RuleDrawer state={state} dispatch={dispatch} />
    </Card>
  );
}

function RuleCard({ item, onEdit, onRemove }) {
  const [sampleInput, setSampleInput] = React.useState("");
  const [run, setRun] = React.useState({
    loading: false,
    result: null,
    error: null,
  });

  const isMask = item.__kind === "mask";
  const opName = isMask ? item.masker : item.validator;
  const paramsStr = item.params
    ? Object.entries(item.params)
        .map(([k, v]) => `${k}=${v == null ? "" : String(v)}`)
        .join(", ")
    : "";

  const handleRun = async () => {
    const input = sampleInput || "";
    setRun({ loading: true, result: null, error: null });
    try {
      let res;
      if (isMask) {
        res = await previewMaskRuleValue(input, item.masker, item.params || null);
      } else {
        res = await previewValidateRuleValue(
          input,
          item.validator,
          item.params || null,
          item.regex || null,
          item.message || null
        );
      }
      setRun({ loading: false, result: res, error: null });
    } catch (e) {
      setRun({ loading: false, result: null, error: String(e) });
    }
  };

  return (
    <List.Item>
      <Space direction="vertical" size="small" style={{ flex: 1, width: "100%" }}>
        <Space size="small" wrap align="center">
          <Tag color={isMask ? "blue" : "gold"} style={{ margin: 0 }}>
            {isMask ? "脱敏" : "校验"}
          </Tag>
          <Text strong>{item.field || "（未指定字段）"}</Text>
          <Text type="secondary">→</Text>
          <Tag style={{ margin: 0 }}>{opName}</Tag>
          {paramsStr ? (
            <Text type="secondary" style={{ fontSize: 12 }}>
              ({paramsStr})
            </Text>
          ) : null}
          {item.tags && item.tags.length ? (
            <Space size={4} wrap>
              {item.tags.map((t) => (
                <Tag key={t} color={tagColor(t)} style={{ margin: 0 }}>
                  {t}
                </Tag>
              ))}
            </Space>
          ) : null}
        </Space>
        {item.description ? (
          <Paragraph
            type="secondary"
            ellipsis={{ rows: 2, expandable: true }}
            style={{ margin: 0, fontSize: 12 }}
          >
            {item.description}
          </Paragraph>
        ) : null}

        {/* 试运行：独立输入样例值，不依赖任何文件 */}
        <div
          style={{
            marginTop: 4,
            padding: "8px 12px",
            background: "#fafafa",
            borderRadius: 4,
          }}
        >
          <Space size="small" wrap align="center" style={{ width: "100%" }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              样例值
            </Text>
            <Input
              size="small"
              style={{ width: 240 }}
              placeholder="输入一个样例值进行试运行"
              value={sampleInput}
              onChange={(e) => setSampleInput(e.target.value)}
              onPressEnter={handleRun}
            />
            <Button
              size="small"
              type="primary"
              icon={<PlayCircleOutlined />}
              loading={run.loading}
              onClick={handleRun}
            >
              试运行
            </Button>
          </Space>

          {run.loading ? (
            <div style={{ marginTop: 8 }}>
              <Space size="small">
                <Spin size="small" />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  正在运行…
                </Text>
              </Space>
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
              {isMask ? (
                <Space direction="vertical" size={4} style={{ width: "100%" }}>
                  <Space size="small" wrap>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      输入
                    </Text>
                    <Text code>{run.result.input}</Text>
                    <ArrowRightOutlined style={{ fontSize: 12, color: "#999" }} />
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      输出
                    </Text>
                    <Text code>{run.result.output}</Text>
                  </Space>
                </Space>
              ) : (
                <Space direction="vertical" size={4} style={{ width: "100%" }}>
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
                      {run.result.valid ? "校验通过" : "校验失败"}
                    </Tag>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      输入
                    </Text>
                    <Text code>{run.result.input}</Text>
                  </Space>
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
      </Space>

      <Space size="small">
        <Tooltip title="编辑规则">
          <span>
            <Button
              type="text"
              icon={<EditOutlined />}
              onClick={() => onEdit(item)}
            />
          </span>
        </Tooltip>
        <Tooltip title="删除规则">
          <span>
            <Button
              type="text"
              danger
              icon={<DeleteOutlined />}
              onClick={() => onRemove(item)}
            />
          </span>
        </Tooltip>
      </Space>
    </List.Item>
  );
}
