import React from "react";
import { AutoComplete } from "antd";

// 字段输入器：规则管理作为独立系统，不强制依赖文件导入。
// - 已导入数据时：headers 作为 AutoComplete 候选，用户可下拉选择或自由输入。
// - 未导入数据时：作为纯输入框，用户输入任意字段名作为规则目标。
//
// 这样规则集可在无数据状态下独立创建/编辑，后续导入数据时再匹配应用。
export default function FieldInput({
  headers = [],
  value,
  onChange,
  placeholder,
  ...rest
}) {
  const options = React.useMemo(
    () => headers.map((h) => ({ value: h, label: h })),
    [headers]
  );
  return (
    <AutoComplete
      value={value}
      options={options}
      onChange={onChange}
      placeholder={placeholder || "输入或选择字段名"}
      filterOption={(input, option) =>
        (option?.value || "").toLowerCase().includes(input.toLowerCase())
      }
      allowClear
      style={{ width: "100%" }}
      {...rest}
    />
  );
}
