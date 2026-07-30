import { Button, Result } from "antd";

// v1.0.0 占位：规则管理能力 v1.2+ 释放。
export default function RulesPanel() {
  return (
    <Result
      status="info"
      title="规则管理"
      subTitle="规则管理能力 v1.2+ 释放"
      extra={<Button disabled>未释放</Button>}
    />
  );
}
