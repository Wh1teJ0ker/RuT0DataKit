import { useEffect, useMemo, useState } from "react";
import {
  Card,
  Radio,
  Button,
  Input,
  Table,
  Tag,
  Space,
  Spin,
  Checkbox,
  Select,
  Empty,
  Typography,
  App as AntApp,
} from "antd";
import {
  saveDialog,
  tauriInvoke,
  extractText,
  extractFile,
  exportExtract,
  listRuleTags,
} from "../tauri.js";

const { TextArea } = Input;
const { Text } = Typography;

// 已知 type 的颜色映射；未知 type 用 default（灰）。
const TYPE_COLORS = {
  phone: "blue",
  bankcard: "green",
  ip: "orange",
  idcard: "purple",
  email: "cyan",
  mac: "magenta",
  username: "gold",
  name: "red",
};

// v0.4.3 T9-5 数据提取视图；v0.4.4 T10-3 加「规则选择」Card。
// 5 Card 结构：
//   ① 输入：Radio 切文件导入 | 文本粘贴
//   ② 规则选择：从 state.rules.validators 勾选要应用的规则（Checkbox.Group）+ tag 过滤
//   ③ 操作：开始提取按钮（按 extractMode + 选中规则构造 rulesJson 调 extractFile/extractText）
//   ④ 结果：顶部计数 Tag（动态遍历 counts）+ antd Table（type/value 两列）
//   ⑤ 导出：三按钮（txt/csv/json）调 saveDialog + exportExtract
// loading 期间禁用按钮防重入；错误走 antd message。
export default function ExtractView({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const {
    rules,
    extractMode,
    extractInput,
    extractResult,
    extractLoading,
    extractSelectedIndices,
    extractRuleTagFilter,
  } = state;

  // tag 候选列表（仿 RulesView），规则库变化时刷新。
  const [tagOptions, setTagOptions] = useState([]);
  useEffect(() => {
    let cancelled = false;
    const rulesJson = JSON.stringify(rules);
    listRuleTags(rulesJson)
      .then((tags) => {
        if (!cancelled) setTagOptions(tags || []);
      })
      .catch(() => {
        if (!cancelled) setTagOptions([]);
      });
    return () => {
      cancelled = true;
    };
  }, [rules]);

  // 规则池：state.rules.validators（校验规则），按 tag 过滤（单值匹配）。
  const filteredRules = useMemo(() => {
    const all = rules.validators || [];
    if (!extractRuleTagFilter) return all;
    return all.filter((r) => r.tag === extractRuleTagFilter);
  }, [rules.validators, extractRuleTagFilter]);

  // Checkbox.Group 的 options：value=规则在 rules.validators 中的全局下标（稳定 key）。
  const ruleOptions = useMemo(() => {
    const all = rules.validators || [];
    return filteredRules.map((r) => {
      const globalIdx = all.indexOf(r);
      const label = (
        <Space size={4}>
          <Tag color={TYPE_COLORS[r.scope] || "default"} style={{ marginRight: 0 }}>
            {r.tag || "validate"}
          </Tag>
          <Text style={{ fontSize: 13 }}>
            {r.scope || "（无 scope）"} -&gt; {r.field || "（无字段）"}
          </Text>
        </Space>
      );
      return { label, value: globalIdx };
    });
  }, [filteredRules, rules.validators]);

  const handleSelectFile = async () => {
    const path = await tauriInvoke("select_file");
    if (path) dispatch({ type: "SET_EXTRACT_INPUT", extractInput: path });
  };

  const handleExtract = async () => {
    if (!extractInput) {
      message.warning("请先选择文件或输入文本");
      return;
    }
    if (extractSelectedIndices.length === 0) {
      message.warning("请至少勾选一条提取规则");
      return;
    }
    // 按选中下标构造 RuleSet JSON（validators 子集，maskers 空）。
    const all = rules.validators || [];
    const selectedRules = extractSelectedIndices
      .map((i) => all[i])
      .filter(Boolean);
    const rulesJson = JSON.stringify({ validators: selectedRules, maskers: [] });

    dispatch({ type: "SET_EXTRACT_LOADING", extractLoading: true });
    try {
      const result =
        extractMode === "file"
          ? await extractFile(extractInput, rulesJson)
          : await extractText(extractInput, rulesJson);
      dispatch({ type: "SET_EXTRACT_RESULT", extractResult: result });
      message.success(`提取完成：${result.findings.length} 条`);
    } catch (e) {
      message.error(`提取失败：${e}`);
    } finally {
      dispatch({ type: "SET_EXTRACT_LOADING", extractLoading: false });
    }
  };

  const handleExport = async (format) => {
    if (!extractResult?.findings?.length) {
      message.warning("无可导出的结果");
      return;
    }
    const outPath = await saveDialog(`extract.${format}`, format);
    if (!outPath) return;
    try {
      await exportExtract(extractResult.findings, format, outPath);
      message.success(`已导出：${outPath}`);
    } catch (e) {
      message.error(`导出失败：${e}`);
    }
  };

  const columns = [
    { title: "类型", dataIndex: "type", key: "type", width: 120 },
    { title: "值", dataIndex: "value", key: "value" },
  ];

  // counts 动态渲染：v0.4.4 后端返回 BTreeMap<String, i64>，不再固定 phone/bankcard/ip。
  const countEntries = useMemo(() => {
    if (!extractResult?.counts) return [];
    return Object.entries(extractResult.counts);
  }, [extractResult]);

  return (
    <Spin spinning={extractLoading}>
      <Card title="输入">
        <Radio.Group
          value={extractMode}
          onChange={(e) =>
            dispatch({ type: "SET_EXTRACT_MODE", extractMode: e.target.value })
          }
          style={{ marginBottom: 16 }}
        >
          <Radio.Button value="file">文件导入</Radio.Button>
          <Radio.Button value="text">文本粘贴</Radio.Button>
        </Radio.Group>
        {extractMode === "file" ? (
          <div>
            <Button onClick={handleSelectFile}>选择 .txt 文件</Button>
            {extractInput && (
              <span style={{ marginLeft: 16, color: "#8c8c8c" }}>
                {extractInput}
              </span>
            )}
          </div>
        ) : (
          <TextArea
            rows={10}
            value={extractInput}
            onChange={(e) =>
              dispatch({
                type: "SET_EXTRACT_INPUT",
                extractInput: e.target.value,
              })
            }
            placeholder="粘贴含手机号/银行卡号/IP/身份证/邮箱 等的文本..."
          />
        )}
      </Card>

      <Card
        title={
          <Space>
            <span>规则选择</span>
            <Tag color="blue">
              {extractSelectedIndices.length} / {rules.validators?.length || 0}
            </Tag>
          </Space>
        }
        style={{ marginTop: 16 }}
      >
        <Space style={{ marginBottom: 12 }}>
          <Select
            size="small"
            style={{ width: 180 }}
            placeholder="按标签过滤"
            value={extractRuleTagFilter ?? "__all__"}
            onChange={(v) =>
              dispatch({
                type: "SET_EXTRACT_RULE_TAG_FILTER",
                extractRuleTagFilter: v === "__all__" ? null : v,
              })
            }
            options={[
              { label: "全部", value: "__all__" },
              ...tagOptions.map((t) => ({ label: t, value: t })),
            ]}
            allowClear
            onClear={() =>
              dispatch({
                type: "SET_EXTRACT_RULE_TAG_FILTER",
                extractRuleTagFilter: null,
              })
            }
          />
          <Text type="secondary" style={{ fontSize: 12 }}>
            勾选要应用的提取规则，提取引擎按每条规则的 scope 查提取正则 + 校验候选
          </Text>
        </Space>
        {ruleOptions.length === 0 ? (
          <Empty
            description={
              (rules.validators?.length || 0) === 0
                ? "规则池为空，内置规则加载失败时此列表为空"
                : `无带「${extractRuleTagFilter}」标签的规则`
            }
          />
        ) : (
          <Checkbox.Group
            value={extractSelectedIndices}
            onChange={(vals) =>
              dispatch({
                type: "SET_EXTRACT_SELECTED_INDICES",
                extractSelectedIndices: vals,
              })
            }
            style={{ display: "flex", flexDirection: "column", gap: 8 }}
          >
            {ruleOptions.map((opt) => (
              <Checkbox key={opt.value} value={opt.value}>
                {opt.label}
              </Checkbox>
            ))}
          </Checkbox.Group>
        )}
      </Card>

      <Card title="操作" style={{ marginTop: 16 }}>
        <Button
          type="primary"
          onClick={handleExtract}
          loading={extractLoading}
          disabled={extractSelectedIndices.length === 0}
        >
          开始提取
        </Button>
        {extractSelectedIndices.length === 0 && (
          <Text type="secondary" style={{ marginLeft: 12 }}>
            请先在上方勾选至少一条规则
          </Text>
        )}
      </Card>

      {extractResult && (
        <Card title="结果" style={{ marginTop: 16 }}>
          <Space style={{ marginBottom: 16 }} wrap>
            {countEntries.length === 0 ? (
              <Tag>无命中</Tag>
            ) : (
              countEntries.map(([type, count]) => (
                <Tag key={type} color={TYPE_COLORS[type] || "default"}>
                  {type}: {count}
                </Tag>
              ))
            )}
          </Space>
          <Table
            dataSource={extractResult.findings}
            columns={columns}
            rowKey={(r, i) => i}
            size="small"
            pagination={{ pageSize: 50 }}
          />
        </Card>
      )}

      {extractResult?.findings?.length > 0 && (
        <Card title="导出" style={{ marginTop: 16 }}>
          <Space>
            <Button onClick={() => handleExport("txt")}>导出 TXT</Button>
            <Button onClick={() => handleExport("csv")}>导出 CSV</Button>
            <Button onClick={() => handleExport("json")}>导出 JSON</Button>
          </Space>
        </Card>
      )}
    </Spin>
  );
}
