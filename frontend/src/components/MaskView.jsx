import React, { useEffect, useMemo, useState } from "react";
import {
  Card,
  Table,
  Button,
  Typography,
  Space,
  Empty,
  Switch,
  Alert,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ArrowRightOutlined,
} from "@ant-design/icons";
import { applyRulesColsRecords } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";
import ColumnRuleMapper from "./ColumnRuleMapper.jsx";

const { Text } = Typography;

// v0.5.0 T12-6：「不脱敏」哨兵、summarizeRule、段②映射表抽到 ColumnRuleMapper。
// 组件不直接 dispatch 映射操作，通过 ColumnRuleMapper 回调把映射操作转成
// dispatch（保持与现有组件一致的字符串字面量 action type）。
const NO_MASK_SENTINEL = "__no_mask__";

// v0.4.0 T5-7：脱敏视图不再各自导入文件，统一消费 PreprocessView 产出的
// state.records。无 records 时渲染 Empty 引导用户先去预处理导入。
// 规则筛选：v0.4.4 规则引擎重构后 tag 为单值字段，r.tag === "mask" 即脱敏侧。
function isMaskRule(r) {
  return r.tag === "mask";
}

// 数据脱敏主视图：四段垂直——①原始数据 ②表头-规则映射 ③预览 ④操作。
// v0.4.2 BUG 3 修正：段②下拉源从「算子模板」（listMaskOpTypes）改为
// 「用户在 RulesView 创建的具体规则」（state.rules.maskers）。用户为每个表头
// 选一条已创建的规则，参数随规则带入，不再在映射表里配参数。要改参数请去
// RulesView 编辑规则。无规则时显示 Alert 引导用户去创建。
export default function MaskView({ state, dispatch }) {
  const { message } = AntApp.useApp();

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
  // v0.5.0 T12-6：抽出 ColumnRuleMapper，本视图只负责传 props + 回调 dispatch。
  // 规则来源：会话级 maskOverrides 优先 + 全局 rules.maskers 按 field 兜底。
  // 临时选规则只写 maskOverrides，不污染全局规则库。
  // v0.4.2 BUG 3：下拉源是「用户创建的具体规则」（taggedMaskers），
  // 不再是算子模板。选规则即把该规则作为 override 写入，参数随规则带入。
  const taggedMaskers = useMemo(
    () => (state.rules.maskers || []).filter(isMaskRule),
    [state.rules.maskers]
  );

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
        <ColumnRuleMapper
          headers={headers}
          rules={taggedMaskers}
          overrides={state.maskOverrides}
          sentinel={NO_MASK_SENTINEL}
          sentinelLabel="不脱敏"
          ruleColumnTitle="脱敏规则"
          noRuleTitle="暂无脱敏规则"
          noRuleDesc="请先到「规则管理」创建脱敏规则，再回到此视图为表头映射规则"
          hint="为每个表头选择已创建的脱敏规则；未选择则该列不脱敏。要改参数请去「规则管理」编辑规则"
          hasRecords={hasRecords}
          onSetOverride={(header, rule) =>
            dispatch({ type: "SET_MASK_OVERRIDE", header, rule })
          }
          onClearOverride={(header) =>
            dispatch({ type: "CLEAR_MASK_OVERRIDE", header })
          }
          onGotoRules={() =>
            dispatch({ type: "SET_VIEW", activeView: "rules" })
          }
        />

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
