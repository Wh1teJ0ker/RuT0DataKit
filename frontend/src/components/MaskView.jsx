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
  Tag,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ArrowRightOutlined,
} from "@ant-design/icons";
import { applyRulesColsRecords } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";
import MaskColumnMapper from "./MaskColumnMapper.jsx";

const { Text } = Typography;

// 「不脱敏」哨兵（保留以兼容旧引用，新映射组件用 allowClear 清除）。
const NO_MASK_SENTINEL = "__no_mask__";

// 数据脱敏主视图：四段垂直——①原始数据 ②表头-脱敏映射（现场选算子+填参数）③预览 ④操作。
// v0.5.x：段②改为 MaskColumnMapper——每列直接选脱敏算子（scope）并填参数，规则
// 作为动态 override 写入 maskOverrides（会话级，不进规则库，导入新文件时清空）。
// 不再依赖「规则管理」全局库，参数随列现场配置。
export default function MaskView({ state, dispatch }) {
  const { message } = AntApp.useApp();

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

  // 段 ② 表头-脱敏映射：每列现场选 scope + 填参数，直接写 maskOverrides。
  // 不再从全局 rules.maskers 兜底——脱敏规则是动态配置，不进规则库。

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
  // 应用时直接用 maskOverrides（每列现场配置的动态规则）组成临时 RuleSet。
  // 全局规则库不被污染。
  const effectiveMaskers = useMemo(() => {
    return Object.values(state.maskOverrides || {}).filter((r) => r && r.scope);
  }, [state.maskOverrides]);

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
          r.summary && r.summary.total_rows != null
            ? `已应用规则，共 ${r.summary.total_rows} 行`
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
              <Tag color="blue" style={{ margin: 0, fontWeight: 600 }}>
                原始数据 {rowCount != null ? rowCount : 0} 行
              </Tag>
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

        {/* 段 ② 表头-脱敏映射（现场选算子 + 填参数） */}
        <MaskColumnMapper
          headers={headers}
          overrides={state.maskOverrides}
          hasRecords={hasRecords}
          onSetOverride={(header, rule) =>
            dispatch({ type: "SET_MASK_OVERRIDE", header, rule })
          }
          onClearOverride={(header) =>
            dispatch({ type: "CLEAR_MASK_OVERRIDE", header })
          }
        />

        {/* 段 ③ 预览 */}
        <Card
          title="脱敏预览"
          styles={{ body: { padding: 12 } }}
          extra={
            <Space size="middle">
              <Tag color="blue" style={{ margin: 0, fontWeight: 600 }}>
                脱敏后{" "}
                {state.maskedRows && state.maskedSummary && state.maskedSummary.total_rows != null
                  ? state.maskedSummary.total_rows
                  : "-"}{" "}
                行
              </Tag>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {state.maskedRows
                  ? state.maskedSummary && state.maskedSummary.total_rows != null
                    ? `已脱敏 ${state.maskedSummary.total_rows} 行`
                    : "已脱敏"
                  : "点击下方应用生成预览"}
              </Text>
            </Space>
          }
        >
          {state.maskedRows ? (
            <>
              {state.maskedSummary && state.maskedSummary.total_rows != null ? (
                <div style={{ marginBottom: 8 }}>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    共 {state.maskedSummary.total_rows} 行
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
