import React, { useMemo } from "react";
import {
  Card,
  Table,
  Button,
  Typography,
  Space,
  Empty,
  Tag,
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ArrowRightOutlined,
} from "@ant-design/icons";
import { runValidateRecords } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";
import ColumnRuleMapper from "./ColumnRuleMapper.jsx";

const { Text } = Typography;

// v0.5.0 T12-6：「不校验」哨兵、summarizeRule、段②映射表抽到 ColumnRuleMapper。
// 组件不直接 dispatch 映射操作，通过 ColumnRuleMapper 回调把映射操作转成
// dispatch（保持与现有组件一致的字符串字面量 action type）。
const NO_VALIDATE_SENTINEL = "__no_validate__";

// v0.4.0 T5-7：校验视图不再各自导入文件，统一消费 PreprocessView 产出的
// state.records。无 records 时渲染 Empty 引导用户先去预处理导入。
// 规则筛选：v0.4.4 规则引擎重构后 tag 为单值字段，r.tag === "validate" 即校验侧。
function isValidateRule(r) {
  return r.tag === "validate";
}

// 数据校验主视图：四段垂直——①原始数据 ②表头-规则映射 ③预览（非法红底）④操作。
// v0.4.2 BUG 3 修正：段②下拉源从「算子模板」（listValidateOpTypes）改为
// 「用户在 RulesView 创建的具体规则」（state.rules.validators）。用户为每个表头
// 选一条已创建的规则，参数随规则带入。要改参数请去 RulesView 编辑规则。
export default function ValidateView({ state, dispatch }) {
  const { message } = AntApp.useApp();

  // v0.4.0 T5-7：数据源来自 PreprocessView 的 state.records（校验不再各自导入文件）。
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
  const rawData = useMemo(() => {
    return rows.slice(0, PREVIEW_ROW_LIMIT).map((row, idx) => {
      const o = { key: idx };
      headers.forEach((h, c) => {
        o[h] = row[c] != null ? row[c] : "";
      });
      return o;
    });
  }, [rows, headers]);

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
  // 规则来源：会话级 validateOverrides 优先 + 全局 rules.validators 按 field 兜底。
  // v0.4.2 BUG 3：下拉源是「用户创建的具体校验规则」（taggedValidators），
  // 不再是算子模板。选规则即把该规则作为 override 写入，参数随规则带入。
  const taggedValidators = useMemo(
    () => (state.rules.validators || []).filter(isValidateRule),
    [state.rules.validators]
  );

  // 段 ③ 预览 Table（非法单元格红底）
  const vrHeaders = useMemo(() => {
    if (!state.validateResult) return [];
    return state.validateResult.headers || headers;
  }, [state.validateResult, headers]);

  const previewData = useMemo(() => {
    if (!state.validateResult || !state.validateResult.rows) return [];
    return state.validateResult.rows.slice(0, PREVIEW_ROW_LIMIT).map((row, idx) => {
      const o = { key: idx };
      vrHeaders.forEach((h, c) => {
        o[h] = row[c] != null ? row[c] : "";
      });
      return o;
    });
  }, [state.validateResult, vrHeaders]);

  const previewColumns = useMemo(() => {
    return vrHeaders.map((h, colIdx) => ({
      title: h,
      key: h,
      render: (_, row, rowIdx) => {
        const valid =
          state.validateResult?.valid_matrix?.[rowIdx]?.[colIdx] !== false;
        const val = row[h] != null ? row[h] : "";
        return (
          <span
            style={{
              background: valid ? "transparent" : "#fff1f0",
              display: "inline-block",
              width: "100%",
              minHeight: 16,
              padding: "0 2px",
            }}
          >
            {val}
          </span>
        );
      },
    }));
  }, [vrHeaders, state.validateResult]);

  const summaryText = useMemo(() => {
    const s = state.validateResult && state.validateResult.summary;
    if (!s) return null;
    const total = s.total_rows != null ? s.total_rows : null;
    const invalidCount = s.invalid_rows != null ? s.invalid_rows : null;
    const validCount =
      total != null && invalidCount != null ? total - invalidCount : null;
    const parts = [];
    if (validCount != null) parts.push(`合法 ${validCount} 行`);
    if (invalidCount != null) parts.push(`非法 ${invalidCount} 行`);
    if (total != null) parts.push(`共 ${total} 行`);
    return parts.length > 0 ? parts.join(" / ") : null;
  }, [state.validateResult]);

  // 段 ④ onRun
  // 应用时合并 override 优先 + 全局 rules 兜底，组成临时 RuleSet。
  // 全局规则库不被污染。v0.4.0 T5-7：全局兜底只取「无标签 或 含 validate 标签」子集，
  // 对应 RuleSet::by_tag("validate") + 无标签规则兼容。
  const effectiveValidators = useMemo(() => {
    const map = new Map();
    for (const r of taggedValidators) {
      map.set(r.field, r);
    }
    for (const r of Object.values(state.validateOverrides || {})) {
      map.set(r.field, r);
    }
    return Array.from(map.values());
  }, [taggedValidators, state.validateOverrides]);

  const onRun = async () => {
    if (!hasRecords) {
      message.warning("请先到数据预处理导入文件");
      return;
    }
    if (effectiveValidators.length === 0) {
      message.warning("请至少为一个表头配置校验算子");
      return;
    }
    dispatch({ type: "SET_LOADING", loading: true });
    dispatch({ type: "SET_HINT", actionHint: "正在运行校验..." });
    try {
      const rulesJson = JSON.stringify({ maskers: [], validators: effectiveValidators });
      const r = await runValidateRecords(records.headers, records.rows, rulesJson);
      dispatch({ type: "SET_VALIDATE", validateResult: r });
      dispatch({ type: "SET_HINT", actionHint: "校验完成" });
    } catch (e) {
      message.error(`校验失败: ${e}`);
      dispatch({ type: "SET_HINT", actionHint: `校验失败: ${e}` });
    } finally {
      dispatch({ type: "SET_LOADING", loading: false });
    }
  };

  return (
    <Card title="数据校验" styles={{ body: { padding: 12 } }}>
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
              <Text type="secondary" style={{ fontSize: 12 }}>
                共 {rowCount} 行 / 显示前 {PREVIEW_ROW_LIMIT} 行
              </Text>
            </Space>
          }
        >
          {hasRecords ? (
            <Table
              size="small"
              pagination={false}
              scroll={{ y: 240, x: "max-content" }}
              sticky
              locale={{ emptyText: "导入文件后此处显示原始数据" }}
              columns={rawColumns}
              dataSource={rawData}
            />
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
          rules={taggedValidators}
          overrides={state.validateOverrides}
          sentinel={NO_VALIDATE_SENTINEL}
          sentinelLabel="不校验"
          ruleColumnTitle="校验规则"
          noRuleTitle="暂无校验规则"
          noRuleDesc="请先到「规则管理」创建校验规则，再回到此视图为表头映射规则"
          hint="为每个表头选择已创建的校验规则；未选择则该列不校验。要改参数请去「规则管理」编辑规则"
          hasRecords={hasRecords}
          onSetOverride={(header, rule) =>
            dispatch({ type: "SET_VALIDATE_OVERRIDE", header, rule })
          }
          onClearOverride={(header) =>
            dispatch({ type: "CLEAR_VALIDATE_OVERRIDE", header })
          }
          onGotoRules={() =>
            dispatch({ type: "SET_VIEW", activeView: "rules" })
          }
        />

        {/* 段 ③ 预览 */}
        <Card
          title="校验预览"
          styles={{ body: { padding: 12 } }}
          extra={
            <Space size="middle">
              <Tag color="blue" style={{ margin: 0, fontWeight: 600 }}>
                {state.validateResult
                  ? `校验后 ${state.validateResult.summary && state.validateResult.summary.total_rows != null ? state.validateResult.summary.total_rows : "-"} 行`
                  : "校验后 - 行"}
              </Tag>
              {state.validateResult && state.validateResult.summary ? (
                (() => {
                  const sm = state.validateResult.summary;
                  const t = sm.total_rows;
                  const inv = sm.invalid_rows;
                  const valid = t != null && inv != null ? t - inv : null;
                  return (
                    <>
                      <Tag color="green" style={{ margin: 0 }}>
                        合法 {valid != null ? valid : "-"}
                      </Tag>
                      <Tag color="red" style={{ margin: 0 }}>
                        非法 {inv != null ? inv : "-"}
                      </Tag>
                    </>
                  );
                })()
              ) : null}
              <Text type="secondary" style={{ fontSize: 12 }}>
                {state.validateResult ? "已运行校验" : "点击下方应用生成预览"}
              </Text>
            </Space>
          }
        >
          {state.validateResult ? (
            <>
              {summaryText ? (
                <div style={{ marginBottom: 8 }}>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {summaryText}
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
              onClick={onRun}
            >
              应用
            </Button>
            <Button
              icon={<ArrowRightOutlined />}
              disabled={!state.validateResult}
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
