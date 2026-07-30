import { Row, Col } from "antd";
import UpdateCard from "./cards/UpdateCard";
import TsharkPathCard from "./cards/TsharkPathCard";
import DbPathCard from "./cards/DbPathCard";
import AboutCard from "./cards/AboutCard";

// 设置页主组件（T8）。
// 渲染 4 张卡片：更新检查 / tshark 路径 / 数据库路径 / 关于。
// 在 Workbench.jsx currentView==='settings' 分支替换 T3 占位 Typography.Text。
// 返回上次 activeCapability 由 T2 的 SET_VIEW action 处理，本组件不持有 state。
export default function SettingsView() {
  return (
    <Row gutter={[16, 16]}>
      <Col xs={24} md={12} xl={8}>
        <UpdateCard />
      </Col>
      <Col xs={24} md={12} xl={8}>
        <TsharkPathCard />
      </Col>
      <Col xs={24} md={12} xl={8}>
        <DbPathCard />
      </Col>
      <Col xs={24} md={12} xl={8}>
        <AboutCard />
      </Col>
    </Row>
  );
}
