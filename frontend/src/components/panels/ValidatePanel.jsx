import { Button, Result } from "antd";

// v1.0.0 占位：校验能力 v1.1+ 释放。
export default function ValidatePanel() {
  return (
    <Result
      status="info"
      title="校验"
      subTitle="校验能力 v1.1+ 释放"
      extra={<Button disabled>未释放</Button>}
    />
  );
}
