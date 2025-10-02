import React from 'react';
import { Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, theme, Button, Space, Typography, Spin } from 'antd';
import {
  DashboardOutlined,
  SettingOutlined,
  DatabaseOutlined,
  MonitorOutlined,
  ApiOutlined,
  ClockCircleOutlined,
  LogoutOutlined,
  UserOutlined,
} from '@ant-design/icons';
import Dashboard from './components/Dashboard';
import DeviceConfig from './components/DeviceConfig';
import EnhancedDeviceConfig from './components/EnhancedDeviceConfig';
import ScheduleGroupConfig from './components/ScheduleGroupConfig';
import DataLogs from './components/DataLogs';
import SystemConfig from './components/SystemConfig';
import PlantConfig from './components/PlantConfig';
import Login from './components/Login';
import { AuthProvider, useAuth } from './contexts/AuthContext';

const { Header, Sider, Content } = Layout;
const { Text } = Typography;

function AuthenticatedApp() {
  const { user, logout, isAdmin } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  
  // Always redirect to dashboard after login if user is on a restricted page
  React.useEffect(() => {
    // If installer is trying to access admin-only pages, redirect to dashboard
    if (!isAdmin && (location.pathname === '/plant-config' || location.pathname === '/config')) {
      navigate('/', { replace: true });
    }
  }, [location.pathname, navigate, isAdmin]);
  
  const {
    token: { colorBgContainer },
  } = theme.useToken();

  const menuItems = [
    {
      key: '/',
      icon: <DashboardOutlined />,
      label: 'Dashboard',
    },
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
  ];

  // Add system config only for admin users
  if (isAdmin) {
    menuItems.push({
      key: '/plant-config',
      icon: <SettingOutlined />,
      label: 'Plant Config',
    });
    menuItems.push({
      key: '/config',
      icon: <SettingOutlined />,
      label: 'System Config',
    });
  }

  const handleMenuClick = (e) => {
    navigate(e.key);
  };

  const handleLogout = async () => {
    await logout();
  };

  return (
    <Layout className="dashboard-layout">
      <Header
        style={{
          padding: 0,
          background: colorBgContainer,
          borderBottom: '1px solid #f0f0f0',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
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
        <div style={{ padding: '0 24px' }}>
          <Space>
            <UserOutlined />
            <Text>{user?.username}</Text>
            <Text type="secondary">({user?.role})</Text>
            <Button 
              type="text" 
              icon={<LogoutOutlined />} 
              onClick={handleLogout}
            >
              Logout
            </Button>
          </Space>
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
            defaultSelectedKeys={[location.pathname]}
            selectedKeys={[location.pathname]}
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
              {isAdmin && <Route path="/plant-config" element={<PlantConfig />} />}
              {isAdmin && <Route path="/config" element={<SystemConfig />} />}
            </Routes>
          </Content>
        </Layout>
      </Layout>
    </Layout>
  );
}

function App() {
  return (
    <AuthProvider>
      <AppContent />
    </AuthProvider>
  );
}

function AppContent() {
  const { user, loading, login } = useAuth();
  const navigate = useNavigate();

  const handleLogin = (userData) => {
    login(userData);
    // Always redirect to dashboard after login
    navigate('/', { replace: true });
  };

  if (loading) {
    return (
      <div style={{ 
        display: 'flex', 
        justifyContent: 'center', 
        alignItems: 'center', 
        minHeight: '100vh' 
      }}>
        <Spin size="large" />
      </div>
    );
  }

  if (!user) {
    return <Login onLogin={handleLogin} />;
  }

  return <AuthenticatedApp />;
}

export default App;
