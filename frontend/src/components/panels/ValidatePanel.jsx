import { Button, Result } from "antd";
import { DEV_STATUS } from "../../constants";

// v1.0.0 占位：校验能力开发中。
export default function ValidatePanel() {
  return (
    <Result
      status="info"
      title="校验"
      subTitle={`校验能力 ${DEV_STATUS}`}
      extra={<Button disabled>未释放</Button>}
    />
  );
}
