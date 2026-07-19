import { Button, Typography, Tag, Space, App as AntApp } from "antd";
import { UploadOutlined } from "@ant-design/icons";
import { tauriInvoke } from "../tauri.js";

const { Text } = Typography;

const TYPE_COLOR = {
  csv: "green",
  xlsx: "blue",
};

// 顶部常驻导入文件工具条。从 props 拿 state/dispatch，不再自管 state。
// 导入成功后 dispatch：
//   SET_FILE（filePath/sourceType/headers/rows/rowCount）
//   SET_SELECTED_COLUMNS（全部数据列）
//   SET_COLUMN_ORDER（headers）
//   SET_EXPORT_FORMAT（"csv"）
export default function FileToolbar({ state, dispatch }) {
  const { message } = AntApp.useApp();
  const { filePath, sourceType, loading } = state;

  const showError = (msg) => message.error(String(msg));

  const handleSelectFile = async () => {
    dispatch({ type: "SET_LOADING", loading: true });
    try {
      const chosen = await tauriInvoke("select_file");
      if (!chosen) {
        dispatch({ type: "SET_LOADING", loading: false });
        return;
      }
      let t = null;
      try {
        t = await tauriInvoke("detect_source_type", { path: chosen });
        if (t !== "csv" && t !== "xlsx") {
          showError(`v0.1.0 仅支持 csv/xlsx，当前识别为 ${t}`);
          dispatch({ type: "SET_LOADING", loading: false });
          return;
        }
      } catch (e) {
        showError(`类型识别失败: ${e}`);
        dispatch({ type: "SET_LOADING", loading: false });
        return;
      }
      try {
        const res = await tauriInvoke("load_preview", { path: chosen });
        const headers = res.headers || [];
        const rows = res.rows || [];
        const rowCount = res.row_count != null ? res.row_count : rows.length;
        dispatch({
          type: "SET_FILE",
          filePath: chosen,
          sourceType: t,
          headers,
          rows,
          rowCount,
        });
        dispatch({ type: "SET_SELECTED_COLUMNS", selectedColumns: headers });
        dispatch({ type: "SET_COLUMN_ORDER", columnOrder: headers });
        dispatch({ type: "SET_EXPORT_FORMAT", exportFormat: "csv" });
        dispatch({ type: "SET_HINT", actionHint: "" });
      } catch (e) {
        showError(`加载预览失败: ${e}`);
      }
    } catch (e) {
      showError(String(e));
    } finally {
      dispatch({ type: "SET_LOADING", loading: false });
    }
  };

  const typeLabel =
    sourceType === null ? null : sourceType === "error" ? "识别失败" : sourceType;
  const tagColor =
    sourceType === null || sourceType === "error"
      ? "default"
      : TYPE_COLOR[sourceType] || "default";

  return (
    <Space size="middle" align="center" wrap>
      <Button
        type="primary"
        icon={<UploadOutlined />}
        loading={loading}
        onClick={handleSelectFile}
      >
        导入文件
      </Button>
      <Text
        type={filePath ? undefined : "secondary"}
        style={{
          flex: 1,
          minWidth: 200,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {filePath || "未选择文件"}
      </Text>
      <Tag color={tagColor}>
        {sourceType === null ? "类型: -" : `类型: ${typeLabel}`}
      </Tag>
    </Space>
  );
}
