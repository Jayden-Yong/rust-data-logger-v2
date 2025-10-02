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
import { useAuth } from '../contexts/AuthContext';

const Dashboard = () => {
  const { isAdmin } = useAuth();
  const [devices, setDevices] = useState([]);
  const [deviceModels, setDeviceModels] = useState([]);
  const [systemStatus, setSystemStatus] = useState(null);
  const [recentLogs, setRecentLogs] = useState([]);
  const [loading, setLoading] = useState(true);
  const [deviceTags, setDeviceTags] = useState({}); // Store all device tags for scaling lookup

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000); // Refresh every 5 seconds
    return () => clearInterval(interval);
  }, []);

  const fetchData = async () => {
    try {
      // Use different endpoint based on user role
      const devicesEndpoint = isAdmin ? '/api/devices-enhanced' : '/api/devices-filtered';
      
      const [devicesRes, modelsRes, statusRes, logsRes] = await Promise.all([
        axios.get(devicesEndpoint),
        axios.get('/api/device-models'),
        axios.get('/api/status'),
        axios.get('/api/logs?limit=10')
      ]);

      if (devicesRes.data.success) {
        const devicesWithStatus = devicesRes.data.data.map(deviceData => ({
          ...deviceData.device,
          tags: deviceData.tags,
          status: deviceData.status,
          is_running: deviceData.is_running,
          last_update: deviceData.last_update,
        }));
        console.log(devicesWithStatus);
        setDevices(devicesWithStatus);
        
        // Fetch device tags for each device to build scaling lookup
        fetchDeviceTags(devicesWithStatus);
      } else if (!isAdmin && devicesRes.data.error) {
        // Handle plant configuration errors for installers
        console.warn('Plant configuration issue for installer:', devicesRes.data.error);
        setDevices([]); // Show empty state for installers with configuration issues
      }

      if (modelsRes.data.success) {
        setDeviceModels(modelsRes.data.data);
      }

      if (statusRes.data.success) {
        setSystemStatus(statusRes.data.data);
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

  const fetchDeviceTags = async (deviceList) => {
    try {
      const tagPromises = deviceList.map(device => 
        axios.get(`/api/devices/${device.id}/tags`)
      );
      
      const tagResponses = await Promise.all(tagPromises);
      const tagsLookup = {};
      
      tagResponses.forEach((response, index) => {
        if (response.data.success) {
          const deviceId = deviceList[index].id;
          tagsLookup[deviceId] = {};
          
          response.data.data.forEach(tag => {
            if (tagsLookup[deviceId][tag.name]) {
              console.log(`Duplicate tag name ${tag.name} for device ${deviceId}, replacing previous entry`);
            }
            tagsLookup[deviceId][tag.name] = {
              scaling_multiplier: tag.scaling_multiplier,
              scaling_offset: tag.scaling_offset,
              unit: tag.unit,
              address: tag.address // Adding address for debugging
            };
          });
        }
      });
      
      setDeviceTags(tagsLookup);
      console.log('Device tags lookup populated:', tagsLookup);
    } catch (error) {
      console.error('Error fetching device tags:', error);
    }
  };

  const handleDeviceAction = async (deviceId, action) => {
    try {
      await axios.post(`/api/devices-enhanced/${deviceId}/${action}`);
      fetchData(); // Refresh data
    } catch (error) {
      console.error(`Error ${action} device:`, error);
    }
  };

  const getDeviceName = (deviceId) => {
    const device = devices.find(d => d.id === deviceId);
    return device ? device.name : deviceId;
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
      title: 'Device Name',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: 'Device Model',
      dataIndex: 'model_id',
      key: 'model_id',
      render: (modelId) => {
        const model = deviceModels.find(m => m.id === modelId);
        return model ? model.name : 'Unknown';
      },
    },
    {
      title: 'Protocol',
      dataIndex: 'protocol_config',
      key: 'protocol',
      render: (protocolConfig) => {
        try {
          const config = JSON.parse(protocolConfig);
          return config?.type?.toUpperCase() || 'Unknown';
        } catch (error) {
          return 'Unknown';
        }
      },
    },
    {
      title: 'Status',
      key: 'status',
      render: (_, record) => {
        let status, color, icon;
        
        if (!record.enabled) {
          status = 'Disabled';
          color = 'default';
          icon = getStatusIcon('Disconnected');
        } else if (record.is_running) {
          status = record.status || 'Running';
          color = record.status === 'Error' ? 'error' : 'success';
          icon = getStatusIcon(record.status || 'Connected');
        } else {
          status = 'Stopped';
          color = 'warning';
          icon = getStatusIcon('Disconnected');
        }
        
        return (
          <Space>
            {icon}
            <Tag color={color}>{status}</Tag>
          </Space>
        );
      },
    },
    {
      title: 'Tags Count',
      dataIndex: 'tags',
      key: 'tags_count',
      render: (tags) => tags?.length || 0,
    },
    {
      title: 'Last Update',
      dataIndex: 'last_update',
      key: 'last_update',
      render: (lastUpdate) => lastUpdate ? moment(lastUpdate).format('YYYY-MM-DD HH:mm:ss') : 'Never',
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space>
          {record.is_running ? (
            <Button
              icon={<PauseCircleOutlined />}
              onClick={() => handleDeviceAction(record.id, 'stop')}
              size="small"
            >
              Stop
            </Button>
          ) : (
            <Button
              icon={<PlayCircleOutlined />}
              onClick={() => handleDeviceAction(record.id, 'start')}
              size="small"
              type="primary"
              disabled={!record.enabled}
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
      render: (deviceId) => getDeviceName(deviceId),
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
      render: (value, record) => {
        // Get scaling factors for this device/tag combination
        const deviceId = record.device_id;
        const tagName = record.tag_name;
        
        let decodedValue = value; // Default to raw value
        let actualUnit = '';
        
        if (deviceTags[deviceId] && deviceTags[deviceId][tagName]) {
          const tagInfo = deviceTags[deviceId][tagName];
          console.log(`Decoding ${tagName}: raw=${value}, multiplier=${tagInfo.scaling_multiplier}, offset=${tagInfo.scaling_offset}`);
          // Apply scaling: decoded_value = (raw_value / scaling_multiplier) + scaling_offset
          // Note: Based on the database schema, it looks like we should divide by multiplier
          decodedValue = (value / tagInfo.scaling_multiplier) + tagInfo.scaling_offset;
          console.log(`Decoded ${tagName}: ${decodedValue}`);
          
          // Get the actual unit (filter out register types)
          const registerTypes = ['input', 'holding', 'coil', 'discrete_input'];
          if (tagInfo.unit && !registerTypes.includes(tagInfo.unit)) {
            actualUnit = tagInfo.unit;
          }
        } else {
          console.log(`No scaling info found for ${deviceId}/${tagName}, showing raw value: ${value}`);
          // Fallback to record.unit if tag info not available
          const registerTypes = ['input', 'holding', 'coil', 'discrete_input'];
          actualUnit = record.unit && !registerTypes.includes(record.unit) ? record.unit : '';
        }
        
        return `${decodedValue.toFixed(2)}${actualUnit ? ' ' + actualUnit : ''}`;
      },
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

  const connectedDevices = devices?.filter(d => d.enabled && d.is_running).length || 0;
  const totalDevices = devices?.length || 0;
  const disabledDevices = devices?.filter(d => !d.enabled).length || 0;
  const errorDevices = devices?.filter(d => d.enabled && d.status === 'Error').length || 0;
  const totalTags = devices?.reduce((sum, device) => sum + (device.tags?.length || 0), 0) || 0;

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
              title="Running Devices"
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
              title="Total Tags"
              value={totalTags}
              prefix={<DatabaseOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col span={24}>
          <Card title="Device Status" className="status-card">
            <Table
              dataSource={devices || []}
              columns={deviceColumns}
              loading={loading}
              rowKey="id"
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
