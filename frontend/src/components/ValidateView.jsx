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
  App as AntApp,
} from "antd";
import {
  PlayCircleOutlined,
  ArrowRightOutlined,
  DeleteOutlined,
} from "@ant-design/icons";
import { VALIDATOR_DEFS, getValidatorDef } from "../validatorDefs.js";
import { runValidateRecords, listValidateOpTypes } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";
import ParamField, { buildDefaultParams } from "./ParamField.jsx";

const { Text } = Typography;

// 「不校验」选项的哨兵值。放在下拉源第一项，避免空字符串在 antd Select
// 中触发 allowClear 警告，也避免与真实算子名冲突。与 MaskView 的 __no_mask__
// 哨兵保持对称，确保两个映射表结构一致。
const NO_VALIDATE_SENTINEL = "__no_validate__";

// v0.4.0 T5-7：校验视图不再各自导入文件，统一消费 PreprocessView 产出的
// state.records。无 records 时渲染 Empty 引导用户先去预处理导入。
// 规则筛选：全局 rules.validators 中带 "validate" 标签的规则归校验侧；无标签
// 规则默认归 validate（向后兼容旧 ruleset）。会话级 validateOverrides 仍按 header 合并。
function isValidateRule(r) {
  const tags = r.tags || [];
  return tags.length === 0 || tags.includes("validate");
}

// 数据校验主视图：四段垂直——①原始数据 ②表头-规则映射 ③预览（非法红底）④操作。
// state/dispatch 从 props 透传；规则通过表头-规则映射 Table 直接 dispatch
// ADD_RULE/UPDATE_RULE/REMOVE_RULE（kind:"validate"），不再嵌入 Form/ValidatorList。
export default function ValidateView({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const [opTypes, setOpTypes] = useState([]);

  useEffect(() => {
    let alive = true;
    listValidateOpTypes()
      .then((list) => {
        if (!alive) return;
        if (Array.isArray(list) && list.length > 0) {
          setOpTypes([
            { name: NO_VALIDATE_SENTINEL, label: "不校验" },
            ...list,
          ]);
        } else {
          setOpTypes([
            { name: NO_VALIDATE_SENTINEL, label: "不校验" },
            ...VALIDATOR_DEFS.map((v) => ({ name: v.name, label: v.description })),
          ]);
        }
      })
      .catch(() => {
        if (!alive) return;
        setOpTypes([
          { name: NO_VALIDATE_SENTINEL, label: "不校验" },
          ...VALIDATOR_DEFS.map((v) => ({ name: v.name, label: v.description })),
        ]);
      });
    return () => {
      alive = false;
    };
  }, []);

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
  // 规则来源：会话级 validateOverrides 优先 + 全局 rules.validators 按 field 兜底。
  // 临时选算子只写 validateOverrides，不污染全局规则库。
  const mappingData = useMemo(() => {
    return headers.map((h, idx) => ({ key: idx, header: h }));
  }, [headers]);

  // 取某表头当前生效的规则（override 优先，rules 兜底）。返回 null 表示不校验。
  // v0.4.0 T5-7：全局兜底规则只取「无标签 或 含 validate 标签」的子集（by_tag 语义）。
  const taggedValidators = useMemo(
    () => (state.rules.validators || []).filter(isValidateRule),
    [state.rules.validators]
  );
  const resolveValidator = (header) => {
    if (state.validateOverrides && state.validateOverrides[header]) {
      return state.validateOverrides[header];
    }
    return taggedValidators.find((r) => r.field === header) || null;
  };

  const onChangeOp = (header, newOpName) => {
    if (!newOpName || newOpName === NO_VALIDATE_SENTINEL) {
      // 选择「不校验」= 清除该表头的会话级 override。
      dispatch({ type: "CLEAR_VALIDATE_OVERRIDE", header });
      return;
    }
    const def = getValidatorDef(newOpName);
    let params = buildDefaultParams(def);
    // 若已有 override 或 rules 中有同 header，保留旧 params 兜底。
    const existing = resolveValidator(header);
    if (existing && existing.validator === newOpName && existing.params) {
      const merged = { ...params };
      for (const p of def.params) {
        if (existing.params[p] != null) merged[p] = existing.params[p];
      }
      params = merged;
    }
    const rule = {
      field: header,
      validator: newOpName,
      params,
      description: undefined,
    };
    dispatch({ type: "SET_VALIDATE_OVERRIDE", header, rule });
  };

  const onChangeParam = (header, paramName, value) => {
    const existing = resolveValidator(header);
    if (!existing) return;
    const params = { ...existing.params, [paramName]: value };
    dispatch({
      type: "SET_VALIDATE_OVERRIDE",
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
        const has = !!resolveValidator(row.header);
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
      title: "校验算子",
      dataIndex: "header",
      key: "op",
      width: 220,
      render: (_, row) => {
        const existing = resolveValidator(row.header);
        const value = existing ? existing.validator : NO_VALIDATE_SENTINEL;
        return (
          <Select
            value={value}
            showSearch
            style={{ width: "100%" }}
            options={opTypes.map((o) => ({ label: o.label, value: o.name }))}
            placeholder="选择校验算子"
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
        const existing = resolveValidator(row.header);
        if (!existing) {
          return (
            <Text type="secondary" style={{ fontSize: 12 }}>
              不校验
            </Text>
          );
        }
        const def = getValidatorDef(existing.validator);
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
        const existing = resolveValidator(row.header);
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
                onClick={() =>
                  dispatch({
                    type: "CLEAR_VALIDATE_OVERRIDE",
                    header: row.header,
                  })
                }
              />
            </span>
          </Tooltip>
        );
      },
    },
  ];

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
    const total = s.total != null ? s.total : null;
    const validCount = s.valid_count != null ? s.valid_count : null;
    const invalidCount = s.invalid_count != null ? s.invalid_count : null;
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
            <Text type="secondary" style={{ fontSize: 12 }}>
              共 {rowCount} 行 / 显示前 {PREVIEW_ROW_LIMIT} 行
            </Text>
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
        <Card
          title="表头-规则映射"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              为每个表头选择校验算子；未选择则该列不校验。映射为会话级临时配置，不影响规则库
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
          title="校验预览"
          styles={{ body: { padding: 12 } }}
          extra={
            <Text type="secondary" style={{ fontSize: 12 }}>
              {state.validateResult ? "已运行校验" : "点击下方应用生成预览"}
            </Text>
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
