import { Select } from "antd";

/**
 * ColumnSelect — 共享列选择下拉。
 *
 * 消除各面板重复的 `headers.map((h) => ({ label: h, value: h }))` +
 * `showSearch optionFilterProp="label"` 样板。MaskPanel / ColumnOpsPanel /
 * CryptoPanel / ExtractPanel / ValidatePanel 中均有相同模式。
 *
 * v1.2.0 T94：从各面板重复的 column Select 中抽出。
 *
 * @param {string[]} headers — 当前 sheet 的列头数组
 * @param {string} [placeholder="选择列"] — 占位文案
 * @param {object} [rest] — 透传给 antd Select 的其他 props（allowClear, mode 等）
 */
export default function ColumnSelect({
  headers = [],
  placeholder = "选择列",
  ...rest
}) {
  return (
    <Select
      placeholder={placeholder}
      options={headers.map((h) => ({ label: h, value: h }))}
      showSearch
      optionFilterProp="label"
      {...rest}
    />
  );
}
