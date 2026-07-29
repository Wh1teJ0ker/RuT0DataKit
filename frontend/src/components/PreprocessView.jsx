import React, { useMemo, useState } from "react";
import {
  Card,
  Table,
  Button,
  Typography,
  Space,
  Descriptions,
  Empty,
  Tag,
  Modal,
  Input,
  App as AntApp,
} from "antd";
import {
  UploadOutlined,
  SearchOutlined,
  SafetyCertificateOutlined,
  CheckCircleOutlined,
  EditOutlined,
  DiffOutlined,
  ConsoleSqlOutlined,
} from "@ant-design/icons";
import { tauriInvoke, preprocessFile, parseSqlTool } from "../tauri.js";
import { PREVIEW_ROW_LIMIT } from "../state.js";

const { Text } = Typography;

// 数据预处理主视图（v0.4.0 入口）：
//   ① 顶部导入按钮（调 select_file + preprocess_file）
//   ② Descriptions 概览（源类型 / 行数 / 列数）
//   ③ antd Table 预览（headers + 前 200 行）— v0.6.5 表头可编辑（✏/批量重命名）
//   ④ 底部跳转按钮组（搜索 / 数据脱敏 / 数据校验）
// 导入产物写入 state.records（SET_RECORDS），切 view 不重置；跳转按钮仅 dispatch SET_VIEW。
// v0.5.x：移除 SQL 解析跳转入口（SQL 盲注自动检测 + 跳转 Tools/Sql 子面板一并删除）。
// v0.6.5 T20-1：表头可编辑 + 批量重命名（SET_COLUMN_RENAME 级联 re-key）+ 数据源指示。
// v0.7.0：表头新增「SQL 解析」按钮——将该列全部非空行作为 SqlParseInput[] 提交
// （复用 parseSqlTool 命令），结果预填到 sqlParseResult + sqlParseInput，然后
// 跳转 Tools/Sql 子面板（镜像 Sidebar.jsx:60-62 模式：SET_VIEW + SET_TOOLS_ACTIVE_TAB）。
export default function PreprocessView({ state, dispatch }) {
  const { message, modal } = AntApp.useApp();
  const { records, loading } = state;
  // v0.6.5 T20-1：批量重命名弹窗内的临时编辑态。key=原表头，value=新表头。
  const [renameMap, setRenameMap] = useState({});
  const [renameOpen, setRenameOpen] = useState(false);

  const showError = (msg) => message.error(String(msg));

  // v0.7.0：列级 SQL 解析——取该列全部非空单元格值作为 SqlParseInput[]，
  // 调 parseSqlTool（复用 Tools/Sql 同款命令），结果预填到 sqlParseInput
  // （textarea 多行，每行一条 SQL）+ sqlParseResult，然后跳转 Tools/Sql
  // 子面板（与 Sidebar.jsx:60-62 同款 SET_VIEW + SET_TOOLS_ACTIVE_TAB 模式）。
  // 空列：warning 不跳转；失败：error 不清空已有解析结果。
  const handleColumnSqlParse = async (columnName) => {
    if (!records) return;
    const sqls = [];
    for (const row of records.rows || []) {
      // records.rows 为二维数组（行 → 列），用 header 索引定位该列。
      const c = records.headers.indexOf(columnName);
      if (c < 0) break;
      const v = row[c];
      if (v == null) continue;
      const s = String(v).trim();
      if (s !== "") sqls.push(s);
    }
    if (sqls.length === 0) {
      message.warning(`列「${columnName}」没有非空值可解析`);
      return;
    }
    const inputs = sqls.map((sql) => ({
      sql,
      responseBodySize: null,
      sourceIp: null,
    }));
    try {
      const result = await parseSqlTool(inputs);
      const text = sqls.join("\n");
      dispatch({ type: "SET_SQL_PARSE_INPUT", sqlParseInput: text });
      dispatch({ type: "SET_SQL_PARSE_RESULT", sqlParseResult: result });
      dispatch({ type: "SET_VIEW", activeView: "tools" });
      dispatch({ type: "SET_TOOLS_ACTIVE_TAB", toolsActiveTab: "sql" });
    } catch (e) {
      showError(`SQL 解析失败: ${e}`);
    }
  };

  const handleImport = async () => {
    dispatch({ type: "SET_LOADING", loading: true });
    try {
      const chosen = await tauriInvoke("select_file");
      if (!chosen) {
        dispatch({ type: "SET_LOADING", loading: false });
        return;
      }
      try {
        const res = await preprocessFile(chosen);
        const headers = res.headers || [];
        const rows = res.rows || [];
        const rowCount =
          res.row_count != null ? res.row_count : rows.length;
        const sourceType = res.source_type || null;
        dispatch({
          type: "SET_RECORDS",
          records: { headers, rows, rowCount, sourceType, filePath: chosen },
        });
        dispatch({ type: "SET_HINT", actionHint: "" });
      } catch (e) {
        showError(`预处理失败: ${e}`);
      }
    } catch (e) {
      showError(String(e));
    } finally {
      dispatch({ type: "SET_LOADING", loading: false });
    }
  };

  // v0.6.5 T20-1：单个表头重命名（✏ 按钮）。弹 Modal 收集新名，dispatch 级联。
  const handleRenameOne = (oldName) => {
    let newName = oldName;
    modal.confirm({
      title: "重命名字段",
      content: (
        <div style={{ marginTop: 8 }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            原名：{oldName}
          </Text>
          <Input
            placeholder="新字段名"
            defaultValue={oldName}
            autoFocus
            onChange={(e) => (newName = e.target.value)}
            style={{ marginTop: 8 }}
            onPressEnter={(e) => {
              newName = e.target.value;
              const doc = document.querySelector(".ant-modal-confirm-btns .ant-btn-primary");
              if (doc) doc.click();
            }}
          />
        </div>
      ),
      onOk: () => {
        const v = newName.trim();
        if (!v || v === oldName) return;
        dispatch({
          type: "SET_COLUMN_RENAME",
          renames: [{ oldName, newName: v }],
        });
        message.success(`已重命名「${oldName}」→「${v}」（脱敏/校验结果已清空，请重新运行）`);
      },
    });
  };

  // v0.6.5 T20-1：批量重命名弹窗。列出所有 headers + 对应 Input，
  // 一次性提交所有变更（只取改名了的）。
  const openBatchRename = () => {
    if (!records) return;
    const initMap = {};
    records.headers.forEach((h) => (initMap[h] = h));
    setRenameMap(initMap);
    setRenameOpen(true);
  };

  const submitBatchRename = () => {
    const renames = Object.entries(renameMap)
      .filter(([oldName, newName]) => newName && newName.trim() && newName.trim() !== oldName)
      .map(([oldName, newName]) => ({ oldName, newName: newName.trim() }));
    if (renames.length === 0) {
      message.info("无字段需要重命名");
      setRenameOpen(false);
      return;
    }
    dispatch({ type: "SET_COLUMN_RENAME", renames });
    message.success(`已重命名 ${renames.length} 个字段（脱敏/校验结果已清空，请重新运行）`);
    setRenameOpen(false);
  };

  // 段 ③ 预览 Table：取 records.headers / records.rows 前 PREVIEW_ROW_LIMIT 行。
  const previewData = useMemo(() => {
    if (!records) return [];
    return records.rows.slice(0, PREVIEW_ROW_LIMIT).map((row, idx) => {
      const o = { key: idx };
      records.headers.forEach((h, c) => {
        o[h] = row[c] != null ? row[c] : "";
      });
      return o;
    });
  }, [records]);

  const previewColumns = useMemo(() => {
    if (!records) return [];
    return records.headers.map((h) => ({
      title: (
        <Space size={4} align="center" wrap={false}>
          <span>{h}</span>
          {/* v0.7.0：将该列全部非空行作为 SQL 探针序列解析，跳转 Tools/Sql */}
          <Button
            type="text"
            size="small"
            icon={<ConsoleSqlOutlined />}
            onClick={() => handleColumnSqlParse(h)}
            title="将该列全部非空行作为 SQL 探针序列解析，跳转 Tools/SQL"
          />
          <Button
            type="text"
            size="small"
            icon={<EditOutlined />}
            onClick={() => handleRenameOne(h)}
            title="重命名此字段"
          />
        </Space>
      ),
      dataIndex: h,
      key: h,
      ellipsis: true,
    }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [records]);

  // 段 ④ 跳转按钮组：点击 dispatch SET_VIEW 切到对应 view，records 不丢。
  const jumps = [
    {
      key: "search",
      label: "搜索",
      icon: <SearchOutlined />,
      view: "search",
    },
    {
      key: "mask",
      label: "数据脱敏",
      icon: <SafetyCertificateOutlined />,
      view: "mask",
    },
    {
      key: "validate",
      label: "数据校验",
      icon: <CheckCircleOutlined />,
      view: "validate",
    },
  ];

  return (
    <Card title="数据预处理" styles={{ body: { padding: 12 } }}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {/* 段 ① 导入 */}
        <Card title="导入" styles={{ body: { padding: 12 } }}>
          <Space size="middle" wrap>
            <Button
              type="primary"
              icon={<UploadOutlined />}
              loading={loading}
              onClick={handleImport}
            >
              导入文件
            </Button>
            <Text type="secondary" style={{ fontSize: 12 }}>
              支持 csv / xlsx / sql / json / pcap / log，自动识别源类型
            </Text>
            {/* v0.6.5 T20-1：数据源指示——预处理流（records），与提取流独立 */}
            <Tag color="geekblue" style={{ margin: 0 }}>
              数据源：预处理流
            </Tag>
          </Space>
        </Card>

        {/* 段 ② 概览 */}
        {records ? (
          <Card title="概览" styles={{ body: { padding: 12 } }}>
            <Descriptions size="small" column={3} bordered>
              <Descriptions.Item label="源类型">
                {records.sourceType || "-"}
              </Descriptions.Item>
              <Descriptions.Item label="行数">
                {records.rowCount}
              </Descriptions.Item>
              <Descriptions.Item label="列数">
                {records.headers.length}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        ) : null}

        {/* 段 ③ 预览 */}
        <Card
          title="预览"
          styles={{ body: { padding: 12 } }}
          extra={
            <Space size="middle" wrap>
              <Tag color="blue" style={{ margin: 0, fontWeight: 600 }}>
                总行数 {records && records.rowCount != null ? records.rowCount : "-"} 行
              </Tag>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {records
                  ? `共 ${records.rowCount} 行 / 显示前 ${PREVIEW_ROW_LIMIT} 行`
                  : "导入文件后此处显示预览"}
              </Text>
              {/* v0.6.5 T20-1：批量重命名入口 */}
              <Button
                size="small"
                icon={<DiffOutlined />}
                disabled={!records}
                onClick={openBatchRename}
                title="批量修改字段名"
              >
                批量重命名
              </Button>
            </Space>
          }
        >
          {records ? (
            <Table
              size="small"
              pagination={false}
              scroll={{ y: 360, x: "max-content" }}
              sticky
              columns={previewColumns}
              dataSource={previewData}
            />
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description="点击上方「导入文件」加载预览"
            />
          )}
        </Card>

        {/* 段 ④ 跳转按钮组 */}
        <Card title="跳转" styles={{ body: { padding: 12 } }}>
          <Space size="middle" wrap>
            {jumps.map((j) => (
              <Button
                key={j.key}
                icon={j.icon}
                disabled={!records}
                onClick={() => dispatch({ type: "SET_VIEW", activeView: j.view })}
              >
                {j.label}
              </Button>
            ))}
          </Space>
          <div style={{ marginTop: 8 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {records
                ? "点击跳转后预处理产物保留，对应视图可直接消费"
                : "请先导入文件后启用跳转"}
            </Text>
          </div>
        </Card>
      </Space>

      {/* v0.6.5 T20-1：批量重命名弹窗 */}
      <Modal
        title="批量重命名字段"
        open={renameOpen}
        onOk={submitBatchRename}
        onCancel={() => setRenameOpen(false)}
        okText="应用重命名"
        cancelText="取消"
        width={520}
      >
        <Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 12 }}>
          修改字段名后会级联更新脱敏/校验/导出/搜索的列引用，并清空已有脱敏/校验结果（需重新运行）。
        </Text>
        <Space direction="vertical" size="small" style={{ width: "100%" }}>
          {Object.entries(renameMap).map(([oldName, newName]) => (
            <Space key={oldName} size="small" style={{ width: "100%" }}>
              <Input
                value={oldName}
                disabled
                style={{ width: "45%" }}
                size="small"
              />
              <Text type="secondary">→</Text>
              <Input
                value={newName}
                onChange={(e) =>
                  setRenameMap((m) => ({ ...m, [oldName]: e.target.value }))
                }
                style={{ width: "45%" }}
                size="small"
                placeholder={oldName}
              />
            </Space>
          ))}
        </Space>
      </Modal>
    </Card>
  );
}
