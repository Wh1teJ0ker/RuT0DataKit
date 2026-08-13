import { Select } from "antd";

/**
 * PhonePrefixSelect — 共享手机号前缀白名单 tags 输入。
 *
 * 用于 MaskPanel（T85 先校验再脱敏 phone-validate）、ExtractPanel
 * （T78 phone-extract 前缀白名单）、ValidatePanel（T78 phone-validate
 * 行级前缀）。三处均使用 mode="tags" + 三位数字过滤模式。
 *
 * v1.2.0 T94：从各面板重复的 phone-prefix Select 中抽出。
 *
 * @param {number} [maxTagCount=5] — 最大标签显示数
 * @param {object} [rest] — 透传给 antd Select 的其他 props
 */
export default function PhonePrefixSelect({
  maxTagCount = 5,
  ...rest
}) {
  return (
    <Select
      mode="tags"
      placeholder="如 134、159"
      tokenSeparators={[",", "，"]}
      maxTagCount={maxTagCount}
      {...rest}
    />
  );
}
