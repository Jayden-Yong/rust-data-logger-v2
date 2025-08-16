import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Table, 
  Select, 
  DatePicker, 
  Button, 
  Space, 
  Tag,
  Statistic,
  Row,
  Col 
} from 'antd';
import { 
  DownloadOutlined, 
  ReloadOutlined,
  DatabaseOutlined 
} from '@ant-design/icons';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import axios from 'axios';
import moment from 'moment';

const { Option } = Select;
const { RangePicker } = DatePicker;

const DataLogs = () => {
  const [logs, setLogs] = useState([]);
  const [devices, setDevices] = useState([]);
  const [loading, setLoading] = useState(true);
  const [selectedDevice, setSelectedDevice] = useState(null);
  const [pagination, setPagination] = useState({
    current: 1,
    pageSize: 50,
    total: 0,
  });

  useEffect(() => {
    fetchDevices();
    fetchLogs();
  }, []);

  useEffect(() => {
    fetchLogs();
  }, [selectedDevice, pagination.current, pagination.pageSize]);

  const fetchDevices = async () => {
    try {
      const response = await axios.get('/api/devices');
      if (response.data.success) {
        setDevices(response.data.data);
      }
    } catch (error) {
      console.error('Error fetching devices:', error);
    }
  };

  const fetchLogs = async () => {
    setLoading(true);
    try {
      const params = new URLSearchParams({
        limit: pagination.pageSize.toString(),
        offset: ((pagination.current - 1) * pagination.pageSize).toString(),
      });

      const url = selectedDevice 
        ? `/api/logs/${selectedDevice}?${params}`
        : `/api/logs?${params}`;

      const response = await axios.get(url);
      if (response.data.success) {
        setLogs(response.data.data);
        // Note: In a real implementation, you'd get the total count from the API
        setPagination(prev => ({ ...prev, total: response.data.data.length }));
      }
    } catch (error) {
      console.error('Error fetching logs:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleDeviceChange = (deviceId) => {
    setSelectedDevice(deviceId);
    setPagination(prev => ({ ...prev, current: 1 }));
  };

  const handleTableChange = (paginationInfo) => {
    setPagination(paginationInfo);
  };

  const handleExport = () => {
    // In a real implementation, this would trigger a CSV/Excel export
    console.log('Exporting data...');
  };

  const columns = [
    {
      title: 'Timestamp',
      dataIndex: 'timestamp',
      key: 'timestamp',
      render: (time) => moment(time).format('YYYY-MM-DD HH:mm:ss'),
      sorter: true,
    },
    {
      title: 'Device',
      dataIndex: 'device_id',
      key: 'device_id',
      filters: devices.map(device => ({
        text: device.name,
        value: device.id,
      })),
      filterMultiple: false,
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
      render: (value, record) => (
        <span>
          {typeof value === 'number' ? value.toFixed(3) : value}
          {record.unit && ` ${record.unit}`}
        </span>
      ),
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
  ];

  // Prepare chart data for the last 100 numeric values
  const chartData = logs
    .filter(log => log.quality === 'Good' && typeof log.value === 'number')
    .slice(-100)
    .map((log, index) => ({
      time: moment(log.timestamp).format('HH:mm:ss'),
      value: log.value,
      tag: log.tag_name,
    }));

  // Calculate statistics
  const goodQualityLogs = logs.filter(log => log.quality === 'Good').length;
  const badQualityLogs = logs.filter(log => log.quality !== 'Good').length;
  const uniqueTags = new Set(logs.map(log => log.tag_name)).size;

  return (
    <div>
      <Row gutter={[16, 16]}>
        <Col span={6}>
          <Card>
            <Statistic
              title="Total Logs"
              value={logs.length}
              prefix={<DatabaseOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Good Quality"
              value={goodQualityLogs}
              valueStyle={{ color: '#3f8600' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Bad Quality"
              value={badQualityLogs}
              valueStyle={{ color: '#cf1322' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Unique Tags"
              value={uniqueTags}
            />
          </Card>
        </Col>
      </Row>

      <Card 
        title="Data Visualization" 
        style={{ marginTop: 16 }}
      >
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={chartData}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="time" />
            <YAxis />
            <Tooltip />
            <Line 
              type="monotone" 
              dataKey="value" 
              stroke="#1890ff" 
              strokeWidth={2}
              dot={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </Card>

      <Card
        title="Data Logs"
        style={{ marginTop: 16 }}
        extra={
          <Space>
            <Select
              placeholder="Filter by device"
              style={{ width: 200 }}
              onChange={handleDeviceChange}
              allowClear
            >
              {devices.map(device => (
                <Option key={device.id} value={device.id}>
                  {device.name}
                </Option>
              ))}
            </Select>
            <RangePicker
              showTime
              format="YYYY-MM-DD HH:mm:ss"
              placeholder={['Start Time', 'End Time']}
            />
            <Button
              icon={<ReloadOutlined />}
              onClick={fetchLogs}
            >
              Refresh
            </Button>
            <Button
              icon={<DownloadOutlined />}
              onClick={handleExport}
            >
              Export
            </Button>
          </Space>
        }
      >
        <Table
          dataSource={logs}
          columns={columns}
          loading={loading}
          rowKey={(record) => `${record.device_id}-${record.tag_name}-${record.timestamp}`}
          pagination={{
            ...pagination,
            showSizeChanger: true,
            showQuickJumper: true,
            showTotal: (total, range) =>
              `${range[0]}-${range[1]} of ${total} items`,
          }}
          onChange={handleTableChange}
          size="small"
        />
      </Card>
    </div>
  );
};

export default DataLogs;
