import React, { useEffect, useMemo, useState } from "react";
import {
  Card,
  Table,
  Select,
  Button,
  Typography,
  Space,
  Empty,
  Tooltip,
  Switch,
  Alert,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ArrowRightOutlined,
  DeleteOutlined,
} from "@ant-design/icons";
import { MASKER_DEFS, getMaskerDef } from "../maskerDefs.js";
import { applyRulesColsRecords, listMaskOpTypes } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";
import ParamField, { buildDefaultParams } from "./ParamField.jsx";

const { Text } = Typography;

// 「不脱敏」选项的哨兵值。放在下拉源第一项，避免空字符串在 antd Select
// 中触发 allowClear 警告，也避免与真实算子名冲突。
const NO_MASK_SENTINEL = "__no_mask__";

// v0.4.0 T5-7：脱敏视图不再各自导入文件，统一消费 PreprocessView 产出的
// state.records。无 records 时渲染 Empty 引导用户先去预处理导入。
// 规则筛选：全局 rules.maskers 中带 "mask" 标签的规则归脱敏侧；无标签规则
// 默认归 mask（向后兼容旧 ruleset）。会话级 maskOverrides 仍按 header 合并。
function isMaskRule(r) {
  const tags = r.tags || [];
  return tags.length === 0 || tags.includes("mask");
}

// 数据脱敏主视图：四段垂直——①原始数据 ②表头-规则映射 ③预览 ④操作。
// state/dispatch 从 props 透传；规则通过表头-规则映射 Table 直接 dispatch
// ADD_RULE/UPDATE_RULE/REMOVE_RULE（kind:"mask"），不再嵌入独立规则表单组件。
export default function MaskView({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [opTypes, setOpTypes] = useState([]);

  useEffect(() => {
    let alive = true;
    listMaskOpTypes()
      .then((list) => {
        if (!alive) return;
        if (Array.isArray(list) && list.length > 0) {
          setOpTypes(list.map((o) => ({ name: o.name, label: o.label })));
        } else {
          setOpTypes(
            MASKER_DEFS.map((m) => ({ name: m.name, label: m.description }))
          );
        }
      })
      .catch(() => {
        if (!alive) return;
        setOpTypes(
          MASKER_DEFS.map((m) => ({ name: m.name, label: m.description }))
        );
      });
    return () => {
      alive = false;
    };
  }, []);

  // v0.4.0 T5-7：数据源来自 PreprocessView 的 state.records（脱敏不再各自导入文件）。
  // 旧 SET_FILE 路径（state.headers/rows）保留兼容，但优先读 records；为空则
  // 渲染 Empty 引导用户回到预处理视图导入文件。
  const records = state.records || null;
  const hasRecords =
    records && Array.isArray(records.headers) && records.headers.length > 0;
  const headers = hasRecords ? records.headers : [];
  const rows = hasRecords ? records.rows : [];
  const rowCount = hasRecords && records.rowCount != null
    ? records.rowCount
    : rows.length;

  // 段 ① 原始数据 Table
  // v0.4.0 T5-13：消费 SearchView 跳转携带的 filteredRowIndices（命中行号集合）。
  // 当用户开启「仅显示搜索命中行」开关时，rawData 按 filteredRowIndices 过滤，
  // 让用户在 MaskView 直接看到搜索命中的原始行（仍可配置脱敏算子、应用规则）。
  const hasFiltered = Array.isArray(state.filteredRowIndices) && state.filteredRowIndices.length > 0;
  const [onlyFiltered, setOnlyFiltered] = useState(false);
  const effectiveRows = useMemo(() => {
    if (onlyFiltered && hasFiltered) {
      const set = new Set(state.filteredRowIndices);
      return rows.filter((_, idx) => set.has(idx));
    }
    return rows;
  }, [rows, onlyFiltered, hasFiltered, state.filteredRowIndices]);

  const rawData = useMemo(() => {
    return effectiveRows.slice(0, PREVIEW_ROW_LIMIT).map((row, idx) => {
      const o = { key: idx };
      headers.forEach((h, c) => {
        o[h] = row[c] != null ? row[c] : "";
      });
      return o;
    });
  }, [effectiveRows, headers]);

  const rawColumns = useMemo(() => {
    return headers.map((h) => ({
      title: h,
      dataIndex: h,
      key: h,
      ellipsis: true,
    }));
  }, [headers]);

  // 段 ② 表头-规则映射 Table
  // 规则来源：会话级 maskOverrides 优先 + 全局 rules.maskers 按 field 兜底。
  // 临时选算子只写 maskOverrides，不污染全局规则库。
  const mappingData = useMemo(() => {
    return headers.map((h, idx) => ({ key: idx, header: h }));
  }, [headers]);

  // 取某表头当前生效的规则（override 优先，rules 兜底）。返回 null 表示不脱敏。
  // v0.4.0 T5-7：全局兜底规则只取「无标签 或 含 mask 标签」的子集（by_tag_mask 语义）。
  const taggedMaskers = useMemo(
    () => (state.rules.maskers || []).filter(isMaskRule),
    [state.rules.maskers]
  );
  const resolveMask = (header) => {
    if (state.maskOverrides && state.maskOverrides[header]) {
      return state.maskOverrides[header];
    }
    return taggedMaskers.find((r) => r.field === header) || null;
  };

  const onChangeOp = (header, newOpName) => {
    if (!newOpName || newOpName === NO_MASK_SENTINEL) {
      // 选择「不脱敏」= 清除该表头的会话级 override。
      dispatch({ type: "CLEAR_MASK_OVERRIDE", header });
      return;
    }
    const def = getMaskerDef(newOpName);
    let params = buildDefaultParams(def);
    // 若已有 override 或 rules 中有同 header，保留旧 params 兜底。
    const existing = resolveMask(header);
    if (existing && existing.masker === newOpName && existing.params) {
      const merged = { ...params };
      for (const p of def.params) {
        if (existing.params[p] != null) merged[p] = existing.params[p];
      }
      params = merged;
    }
    const rule = { field: header, masker: newOpName, params, description: undefined };
    dispatch({ type: "SET_MASK_OVERRIDE", header, rule });
  };

  const onChangeParam = (header, paramName, value) => {
    const existing = resolveMask(header);
    if (!existing) return;
    const params = { ...existing.params, [paramName]: value };
    dispatch({
      type: "SET_MASK_OVERRIDE",
      header,
      rule: { ...existing, params },
    });
  };

  const mappingColumns = [
    {
      title: "表头",
      dataIndex: "header",
      width: 160,
      render: (text, row) => {
        const has = !!resolveMask(row.header);
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
      title: "脱敏算子",
      dataIndex: "header",
      key: "op",
      width: 220,
      render: (_, row) => {
        const existing = resolveMask(row.header);
        const value = existing ? existing.masker : NO_MASK_SENTINEL;
        return (
          <Select
            value={value}
            showSearch
            style={{ width: "100%" }}
            options={[
              { name: NO_MASK_SENTINEL, label: "不脱敏" },
              ...opTypes,
            ].map((o) => ({ label: o.label, value: o.name }))}
            placeholder="选择脱敏算子"
            onChange={(v) => onChangeOp(row.header, v)}
          />
        );
      },
    },
    {
      title: "参数",
      key: "params",
      width: 380,
      render: (_, row) => {
        const existing = resolveMask(row.header);
        if (!existing) {
          return <Text type="secondary">不脱敏</Text>;
        }
        const def = getMaskerDef(existing.masker);
        if (!def.params || def.params.length === 0) {
          return (
            <Text type="secondary" style={{ fontSize: 12 }}>
              无参数
            </Text>
          );
        }
        return (
          <Space size="small" wrap>
            {def.params.map((p) => (
              <div key={p} style={{ display: "flex", flexDirection: "column" }}>
                <ParamField
                  paramName={p}
                  def={def}
                  value={existing.params?.[p]}
                  onChange={(v) => onChangeParam(row.header, p, v)}
                />
                <Text type="secondary" style={{ fontSize: 10, marginTop: 2 }}>
                  {def.paramDocs?.[p] || p}
                </Text>
              </div>
            ))}
          </Space>
        );
      },
    },
    {
      title: "操作",
      key: "actions",
      width: 80,
      align: "center",
      render: (_, row) => {
        const existing = resolveMask(row.header);
        if (!existing) {
          return <Text type="secondary" style={{ fontSize: 12 }}>—</Text>;
        }
        return (
          <Tooltip title="清除该列映射（不影响规则库）">
            <span>
              <Button
                type="text"
                danger
                size="small"
                icon={<DeleteOutlined />}
                onClick={() =>
                  dispatch({ type: "CLEAR_MASK_OVERRIDE", header: row.header })
                }
              />
            </span>
          </Tooltip>
        );
      },
    },
  ];

  // 段 ③ 预览 Table
  const previewData = useMemo(() => {
    if (!state.maskedRows) return [];
    return state.maskedRows.slice(0, PREVIEW_ROW_LIMIT).map((row, idx) => {
      const o = { key: idx };
      headers.forEach((h, c) => {
        o[h] = row[c] != null ? row[c] : "";
      });
      return o;
    });
  }, [state.maskedRows, headers]);

  const previewColumns = useMemo(() => {
    return headers.map((h) => ({
      title: h,
      dataIndex: h,
      key: h,
      ellipsis: true,
    }));
  }, [headers]);

  // 段 ④ onApply
  // 应用时合并 override 优先 + 全局 rules 兜底，组成临时 RuleSet。
  // 全局规则库不被污染。v0.4.0 T5-7：全局兜底只取「无标签 或 含 mask 标签」子集，
  // 对应 RuleSet::by_tag_mask("mask") + 无标签规则兼容。
  const effectiveMaskers = useMemo(() => {
    const map = new Map();
    // 全局规则库兜底（按 mask 标签筛 + 无标签兼容）
    for (const r of taggedMaskers) {
      map.set(r.field, r);
    }
    // 会话级 override 覆盖
    for (const r of Object.values(state.maskOverrides || {})) {
      map.set(r.field, r);
    }
    return Array.from(map.values());
  }, [taggedMaskers, state.maskOverrides]);

  const onApply = async () => {
    if (!hasRecords) {
      message.warning("请先到数据预处理导入文件");
      return;
    }
    const selectedColumns = effectiveMaskers.map((r) => r.field);
    if (selectedColumns.length === 0) {
      message.warning("请至少为一个表头配置脱敏算子");
      return;
    }
    dispatch({ type: "SET_LOADING", loading: true });
    dispatch({ type: "SET_HINT", actionHint: "正在应用规则..." });
    try {
      const rulesJson = JSON.stringify({ maskers: effectiveMaskers, validators: [] });
      const r = await applyRulesColsRecords(
        records.headers,
        records.rows,
        rulesJson,
        selectedColumns
      );
      dispatch({
        type: "SET_MASKED",
        maskedRows: r.masked_rows,
        maskedSummary: r.summary,
      });
      dispatch({
        type: "SET_HINT",
        actionHint:
          r.summary && r.summary.total != null
            ? `已应用规则，共 ${r.summary.total} 行`
            : "已应用规则",
      });
    } catch (e) {
      message.error(`应用失败: ${e}`);
      dispatch({ type: "SET_HINT", actionHint: `应用失败: ${e}` });
    } finally {
      dispatch({ type: "SET_LOADING", loading: false });
    }
  };

  return (
    <Card title="数据脱敏" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {/* 段 ① 原始数据 */}
        <Card
          title="原始数据"
          styles={{ body: { padding: 12 } }}
          extra={
            <Space size="middle">
              {hasFiltered && (
                <Space size="small">
                  <Switch
                    size="small"
                    checked={onlyFiltered}
                    onChange={setOnlyFiltered}
                  />
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    仅显示搜索命中行（{state.filteredRowIndices.length}）
                  </Text>
                </Space>
              )}
              <Text type="secondary" style={{ fontSize: 12 }}>
                共 {rowCount} 行 / 显示前 {PREVIEW_ROW_LIMIT} 行
              </Text>
            </Space>
          }
        >
          {hasRecords ? (
            <>
              {onlyFiltered && hasFiltered && (
                <Alert
                  type="info"
                  showIcon
                  style={{ marginBottom: 8 }}
                  message={`已按搜索命中行过滤：${state.filteredRowIndices.length} 行`}
                />
              )}
              <Table
              size="small"
              pagination={false}
              scroll={{ y: 240, x: "max-content" }}
              sticky
              locale={{ emptyText: "导入文件后此处显示原始数据" }}
              columns={rawColumns}
              dataSource={rawData}
            />
            </>
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description="请先到数据预处理导入文件"
            />
          )}
        </Card>

        {/* 段 ② 表头-规则映射 */}
        <Card
          title="表头-规则映射"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              为每个表头选择脱敏算子；未选择则该列不脱敏。映射为会话级临时配置，不影响规则库
            </Text>
          }
        >
          {hasRecords ? (
            <Table
              size="small"
              pagination={false}
              scroll={{ y: 240 }}
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

        {/* 段 ③ 预览 */}
        <Card
          title="脱敏预览"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              {state.maskedRows
                ? state.maskedSummary && state.maskedSummary.total != null
                  ? `已脱敏 ${state.maskedSummary.total} 行`
                  : "已脱敏"
                : "点击下方应用生成预览"}
            </Text>
          }
        >
          {state.maskedRows ? (
            <>
              {state.maskedSummary && state.maskedSummary.total != null ? (
                <div style={{ marginBottom: 8 }}>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    共 {state.maskedSummary.total} 行
                  </Text>
                </div>
              ) : null}
              <Table
                size="small"
                pagination={false}
                scroll={{ y: 360, x: "max-content" }}
                sticky
                columns={previewColumns}
                dataSource={previewData}
              />
            </>
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description="点击下方「应用」生成预览"
            />
          )}
        </Card>

        {/* 段 ④ 操作 */}
        <Card title="操作" styles={{ body: { padding: 12 } }}>
          <Space size="middle" wrap>
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              loading={state.loading}
              onClick={onApply}
            >
              应用
            </Button>
            <Button
              icon={<ArrowRightOutlined />}
              disabled={!state.maskedRows}
              onClick={() => dispatch({ type: "SET_VIEW", activeView: "export" })}
            >
              导出
            </Button>
          </Space>
          <div style={{ marginTop: 8 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {state.actionHint}
            </Text>
          </div>
        </Card>
      </Space>
    </Card>
  );
}
