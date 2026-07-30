import { Button, Result } from "antd";

// v1.0.0 占位：脱敏能力 v1.1+ 释放（见 docs/versions/1.0.0/规划需求.md §2.2）。
export default function MaskPanel() {
  return (
    <Result
      status="info"
      title="脱敏"
      subTitle="脱敏能力 v1.1+ 释放"
      extra={<Button disabled>未释放</Button>}
    />
  );
}
