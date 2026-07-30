import { Layout } from "antd";

const { Header, Content } = Layout;

// v1.0.0: 仅 antd Layout 占位，验证 Tauri ↔ Vite dev 链路通。
// 四区布局（TopToolbar / SidePanel / Workbench / AiPanel）由 T2 实现。
export default function App() {
  return (
    <Layout style={{ height: "100vh" }}>
      <Header style={{ color: "#fff" }}>RuT0DataKit</Header>
      <Content style={{ padding: 24 }}>框架占位</Content>
    </Layout>
  );
}
