import { Button, Result } from "antd";
import { DEV_STATUS } from "../../constants";

// v1.0.0 占位：规则管理能力开发中。
export default function RulesPanel() {
  return (
    <Result
      status="info"
      title="规则管理"
      subTitle={`规则管理能力 ${DEV_STATUS}`}
      extra={<Button disabled>未释放</Button>}
    />
  );
}
