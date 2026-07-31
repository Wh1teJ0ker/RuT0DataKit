import { Button, Result } from "antd";
import { DEV_STATUS } from "../../constants";

// v1.0.0 占位：脱敏能力开发中（见 docs/versions/1.0.0/规划需求.md §2.2）。
export default function MaskPanel() {
  return (
    <Result
      status="info"
      title="脱敏"
      subTitle={`脱敏能力 ${DEV_STATUS}`}
      extra={<Button disabled>未释放</Button>}
    />
  );
}
