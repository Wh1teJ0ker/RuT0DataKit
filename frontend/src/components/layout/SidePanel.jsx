import { Button, Empty, Layout } from "antd";
import { SettingOutlined } from "@ant-design/icons";
import MaskPanel from "../panels/MaskPanel";
import ValidatePanel from "../panels/ValidatePanel";
import ExtractPanel from "../panels/ExtractPanel";
import RulesPanel from "../panels/RulesPanel";

const { Sider } = Layout;

const PANELS = {
  mask: MaskPanel,
  validate: ValidatePanel,
  extract: ExtractPanel,
  rules: RulesPanel,
};

// 左侧动态能力面板容器：由 activeCapability 驱动切换 4 个占位面板；
// activeCapability === null 显示空态。底部常驻「⚙ 设置」按钮（点击进设置页，T8 实现）。
export default function SidePanel({ activeCapability, setView }) {
  const PanelComp = activeCapability ? PANELS[activeCapability] : null;

  return (
    <Sider
      width={260}
      theme="light"
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        overflow: "auto",
        borderRight: "1px solid #f0f0f0",
      }}
    >
      <div style={{ flex: 1, padding: 8 }}>
        {PanelComp ? (
          <PanelComp />
        ) : (
          <Empty
            style={{ marginTop: 48 }}
            description="点击上方能力按钮展开面板"
          />
        )}
      </div>
      <div
        style={{
          borderTop: "1px solid #f0f0f0",
          padding: 8,
        }}
      >
        <Button
          block
          icon={<SettingOutlined />}
          onClick={() => setView("settings")}
        >
          设置
        </Button>
      </div>
    </Sider>
  );
}
