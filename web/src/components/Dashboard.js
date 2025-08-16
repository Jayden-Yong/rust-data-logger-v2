import React, { useState, useEffect } from 'react';
import { Row, Col, Card, Statistic, Table, Tag, Button, Space } from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  MonitorOutlined,
  DatabaseOutlined
} from '@ant-design/icons';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import axios from 'axios';
import moment from 'moment';

const Dashboard = () => {
  const [status, setStatus] = useState(null);
  const [recentLogs, setRecentLogs] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000); // Refresh every 5 seconds
    return () => clearInterval(interval);
  }, []);

  const fetchData = async () => {
    try {
      const [statusRes, logsRes] = await Promise.all([
        axios.get('/api/status'),
        axios.get('/api/logs?limit=10')
      ]);

      if (statusRes.data.success) {
        setStatus(statusRes.data.data);
      }

      if (logsRes.data.success) {
        setRecentLogs(logsRes.data.data);
      }
    } catch (error) {
      console.error('Error fetching data:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleDeviceAction = async (deviceId, action) => {
    try {
      await axios.post(`/api/devices/${deviceId}/${action}`);
      fetchData(); // Refresh data
    } catch (error) {
      console.error(`Error ${action} device:`, error);
    }
  };

  const getStatusIcon = (status) => {
    switch (status) {
      case 'Connected':
      case 'Reading':
        return <CheckCircleOutlined className="device-status-good" />;
      case 'Error':
        return <ExclamationCircleOutlined className="device-status-error" />;
      default:
        return <ClockCircleOutlined className="device-status-warning" />;
    }
  };

  const getStatusColor = (status) => {
    switch (status) {
      case 'Connected':
      case 'Reading':
        return 'success';
      case 'Error':
        return 'error';
      default:
        return 'warning';
    }
  };

  const deviceColumns = [
    {
      title: 'Device ID',
      dataIndex: 'device_id',
      key: 'device_id',
    },
    {
      title: 'Status',
      dataIndex: 'status',
      key: 'status',
      render: (status) => (
        <Space>
          {getStatusIcon(status)}
          <Tag color={getStatusColor(status)}>{status}</Tag>
        </Space>
      ),
    },
    {
      title: 'Last Update',
      dataIndex: 'last_update',
      key: 'last_update',
      render: (time) => moment(time).format('YYYY-MM-DD HH:mm:ss'),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space>
          {record.is_running ? (
            <Button
              icon={<PauseCircleOutlined />}
              onClick={() => handleDeviceAction(record.device_id, 'stop')}
              size="small"
            >
              Stop
            </Button>
          ) : (
            <Button
              icon={<PlayCircleOutlined />}
              onClick={() => handleDeviceAction(record.device_id, 'start')}
              size="small"
              type="primary"
            >
              Start
            </Button>
          )}
        </Space>
      ),
    },
  ];

  const logColumns = [
    {
      title: 'Device',
      dataIndex: 'device_id',
      key: 'device_id',
    },
    {
      title: 'Tag',
      dataIndex: 'tag_name',
      key: 'tag_name',
    },
    {
      title: 'Value',
      dataIndex: 'value',
      key: 'value',
      render: (value, record) => 
        `${value.toFixed(2)} ${record.unit || ''}`,
    },
    {
      title: 'Quality',
      dataIndex: 'quality',
      key: 'quality',
      render: (quality) => (
        <Tag color={quality === 'Good' ? 'success' : 'error'}>
          {quality}
        </Tag>
      ),
    },
    {
      title: 'Timestamp',
      dataIndex: 'timestamp',
      key: 'timestamp',
      render: (time) => moment(time).format('HH:mm:ss'),
    },
  ];

  // Prepare chart data
  const chartData = recentLogs
    .filter(log => log.quality === 'Good')
    .slice(-20)
    .map((log, index) => ({
      time: moment(log.timestamp).format('HH:mm:ss'),
      value: log.value,
      name: log.tag_name,
    }));

  const connectedDevices = status?.devices?.filter(d => d.status === 'Connected' || d.status === 'Reading').length || 0;
  const totalDevices = status?.devices?.length || 0;
  const errorDevices = status?.devices?.filter(d => d.status === 'Error').length || 0;

  return (
    <div>
      <Row gutter={[16, 16]}>
        <Col span={6}>
          <Card>
            <Statistic
              title="Total Devices"
              value={totalDevices}
              prefix={<MonitorOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Connected Devices"
              value={connectedDevices}
              prefix={<CheckCircleOutlined />}
              valueStyle={{ color: '#3f8600' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Error Devices"
              value={errorDevices}
              prefix={<ExclamationCircleOutlined />}
              valueStyle={{ color: '#cf1322' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Total Log Entries"
              value={status?.total_log_entries || 0}
              prefix={<DatabaseOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col span={24}>
          <Card title="Device Status" className="status-card">
            <Table
              dataSource={status?.devices || []}
              columns={deviceColumns}
              loading={loading}
              rowKey="device_id"
              pagination={{ pageSize: 5 }}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col span={14}>
          <Card title="Real-time Data Trends">
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="time" />
                <YAxis />
                <Tooltip />
                <Line type="monotone" dataKey="value" stroke="#1890ff" strokeWidth={2} />
              </LineChart>
            </ResponsiveContainer>
          </Card>
        </Col>
        <Col span={10}>
          <Card title="Recent Logs">
            <Table
              dataSource={recentLogs}
              columns={logColumns}
              loading={loading}
              rowKey="id"
              pagination={false}
              size="small"
              scroll={{ y: 300 }}
            />
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default Dashboard;
