import React from 'react';
import { Routes, Route } from 'react-router-dom';
import { Layout, Menu, theme } from 'antd';
import {
  DashboardOutlined,
  SettingOutlined,
  DatabaseOutlined,
  MonitorOutlined,
  ApiOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons';
import Dashboard from './components/Dashboard';
import DeviceConfig from './components/DeviceConfig';
import EnhancedDeviceConfig from './components/EnhancedDeviceConfig';
import ScheduleGroupConfig from './components/ScheduleGroupConfig';
import DataLogs from './components/DataLogs';
import SystemConfig from './components/SystemConfig';

const { Header, Sider, Content } = Layout;

function App() {
  const {
    token: { colorBgContainer },
  } = theme.useToken();

  const menuItems = [
    {
      key: '/',
      icon: <DashboardOutlined />,
      label: 'Dashboard',
    },
    // {
    //   key: '/devices',
    //   icon: <MonitorOutlined />,
    //   label: 'Device Config',
    // },
    {
      key: '/devices',
      icon: <ApiOutlined />,
      label: 'Devices',
    },
    {
      key: '/schedule-groups',
      icon: <ClockCircleOutlined />,
      label: 'Schedule Groups',
    },
    {
      key: '/logs',
      icon: <DatabaseOutlined />,
      label: 'Data Logs',
    },
    {
      key: '/config',
      icon: <SettingOutlined />,
      label: 'System Config',
    },
  ];

  const handleMenuClick = (e) => {
    window.location.href = e.key;
  };

  return (
    <Layout className="dashboard-layout">
      <Header
        style={{
          padding: 0,
          background: colorBgContainer,
          borderBottom: '1px solid #f0f0f0',
        }}
      >
        <div style={{ 
          padding: '0 24px', 
          fontSize: '18px', 
          fontWeight: 'bold',
          color: '#1890ff' 
        }}>
          AVA Device Logger
        </div>
      </Header>
      <Layout>
        <Sider
          width={200}
          style={{
            background: colorBgContainer,
          }}
        >
          <Menu
            mode="inline"
            defaultSelectedKeys={[window.location.pathname]}
            style={{
              height: '100%',
              borderRight: 0,
            }}
            items={menuItems}
            onClick={handleMenuClick}
          />
        </Sider>
        <Layout style={{ padding: '0 24px 24px' }}>
          <Content
            className="dashboard-content"
            style={{
              background: colorBgContainer,
              margin: 0,
              minHeight: 280,
            }}
          >
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/devices" element={<EnhancedDeviceConfig />} />
              <Route path="/schedule-groups" element={<ScheduleGroupConfig />} />
              <Route path="/logs" element={<DataLogs />} />
              <Route path="/config" element={<SystemConfig />} />
            </Routes>
          </Content>
        </Layout>
      </Layout>
    </Layout>
  );
}

export default App;
