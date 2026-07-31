import { Button, Result } from "antd";
import { DEV_STATUS } from "../../constants";

// v1.0.0 占位：提取能力开发中。
export default function ExtractPanel() {
  return (
    <Result
      status="info"
      title="提取"
      subTitle={`提取能力 ${DEV_STATUS}`}
      extra={<Button disabled>未释放</Button>}
    />
  );
}
