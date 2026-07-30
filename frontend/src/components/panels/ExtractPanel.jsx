import { Button, Result } from "antd";

// v1.0.0 占位：提取能力 v1.2+ 释放。
export default function ExtractPanel() {
  return (
    <Result
      status="info"
      title="提取"
      subTitle="提取能力 v1.2+ 释放"
      extra={<Button disabled>未释放</Button>}
    />
  );
}
