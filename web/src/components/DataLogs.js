import React, { useState, useEffect, useCallback } from 'react';
import { 
  Card, 
  Table, 
  Select, 
  Button, 
  Space, 
  Tag,
  Statistic,
  Row,
  Col,
  message 
} from 'antd';
import { 
  DownloadOutlined, 
  ReloadOutlined,
  DatabaseOutlined,
  LineChartOutlined 
} from '@ant-design/icons';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import axios from 'axios';
import moment from 'moment';

const { Option } = Select;

const DataLogs = () => {
  const [logs, setLogs] = useState([]);
  const [devices, setDevices] = useState([]);
  const [loading, setLoading] = useState(true);
  const [selectedDevice, setSelectedDevice] = useState(null);
  const [tableData, setTableData] = useState([]);
  const [chartData, setChartData] = useState([]);
  const [selectedTagForChart, setSelectedTagForChart] = useState(null);
  const [pagination, setPagination] = useState({
    current: 1,
    pageSize: 50,
    total: 0,
  });

  const fetchDevices = async () => {
    try {
      const response = await axios.get('/api/devices-enhanced');
      if (response.data.success) {
        // Extract device info from the enhanced response structure
        const devices = response.data.data.map(item => ({
          id: item.device.id,
          name: item.device.name,
          tags: item.tags || []
        }));
        setDevices(devices);
      }
    } catch (error) {
      console.error('Error fetching devices:', error);
    }
  };

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    try {
      const params = new URLSearchParams({
        limit: pagination.pageSize.toString(),
        offset: ((pagination.current - 1) * pagination.pageSize).toString(),
      });

      // Always fetch all logs, we'll filter on the frontend
      const url = `/api/logs?${params}`;

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
  }, [pagination.current, pagination.pageSize]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    fetchDevices();
    fetchLogs();
  }, [fetchLogs]);

  useEffect(() => {
    // Don't fetch again if already loading or if this is the initial load
    if (!loading) {
      fetchLogs();
    }
  }, [selectedDevice]); // eslint-disable-line react-hooks/exhaustive-deps

  const processTableData = useCallback(async (tags) => {
    if (!selectedDevice) return;
    
    try {
      // Fetch comprehensive logs for the selected device
      const response = await axios.get(`/api/logs/${selectedDevice}`, {
        params: {
          limit: 2000 // Get more comprehensive data for table
        }
      });

      if (response.data.success) {
        const deviceLogs = response.data.data;
        const now = moment();
        const twentyFourHoursAgo = now.clone().subtract(24, 'hours');
        
        const tableRows = tags.map(tag => {
          // Find the latest log entry for this tag within the last 24 hours
          const tagLogs = deviceLogs.filter(log => 
            log.tag_name === tag.name &&
            moment(log.timestamp).isAfter(twentyFourHoursAgo)
          );
          
          // Find the latest log entry within the 24-hour window
          const latestLog = tagLogs.reduce((latest, current) => {
            if (!latest || moment(current.timestamp).isAfter(moment(latest.timestamp))) {
              return current;
            }
            return latest;
          }, null);

          return {
            key: tag.id,
            tag: tag.name,
            value: latestLog ? latestLog.value : 'NA',
            timestamp: latestLog ? moment(latestLog.timestamp).format('YYYY-MM-DD HH:mm:ss') : 'NA',
            quality: latestLog ? latestLog.quality : 'NA',
            unit: tag.unit || '',
            tagName: tag.name
          };
        });

        setTableData(tableRows);
      }
    } catch (error) {
      console.error('Error fetching table data:', error);
      // Fallback to existing logs if API call fails
      const now = moment();
      const twentyFourHoursAgo = now.clone().subtract(24, 'hours');
      
      const tableRows = tags.map(tag => {
        // Find the latest log entry for this tag within the last 24 hours
        const tagLogs = logs.filter(log => 
          log.device_id === selectedDevice && 
          log.tag_name === tag.name &&
          moment(log.timestamp).isAfter(twentyFourHoursAgo)
        );
        
        const latestLog = tagLogs.reduce((latest, current) => {
          if (!latest || moment(current.timestamp).isAfter(moment(latest.timestamp))) {
            return current;
          }
          return latest;
        }, null);

        return {
          key: tag.id,
          tag: tag.name,
          value: latestLog ? latestLog.value : 'NA',
          timestamp: latestLog ? moment(latestLog.timestamp).format('YYYY-MM-DD HH:mm:ss') : 'NA',
          quality: latestLog ? latestLog.quality : 'NA',
          unit: tag.unit || '',
          tagName: tag.name
        };
      });

      setTableData(tableRows);
    }
  }, [selectedDevice, logs]);

  useEffect(() => {
    // Process table data when device is selected
    const loadTableData = async () => {
      if (selectedDevice) {
        const selectedDeviceData = devices.find(d => d.id === selectedDevice);
        if (selectedDeviceData && selectedDeviceData.tags) {
          await processTableData(selectedDeviceData.tags);
        }
      } else {
        setTableData([]);
      }
    };
    
    loadTableData();
  }, [selectedDevice, devices, processTableData]);

  const handleVisualize = async (tagName) => {
    if (!selectedDevice || !tagName) return;
    
    try {
      const now = moment();
      const twentyFourHoursAgo = now.clone().subtract(24, 'hours');
      
      // Fetch more logs for the specific device
      const response = await axios.get(`/api/logs/${selectedDevice}`, {
        params: {
          limit: 1000 // Get more data points for better chart
        }
      });

      if (response.data.success) {
        // Filter for the specific tag and time range
        const tagLogs = response.data.data.filter(log => 
          log.tag_name === tagName &&
          moment(log.timestamp).isAfter(twentyFourHoursAgo)
        ).sort((a, b) => moment(a.timestamp).valueOf() - moment(b.timestamp).valueOf());

        const chartDataForTag = tagLogs.map(log => ({
          time: moment(log.timestamp).format('MM-DD HH:mm'),
          value: log.value,
          timestamp: log.timestamp
        }));

        setChartData(chartDataForTag);
        setSelectedTagForChart(tagName);
        
        if (chartDataForTag.length === 0) {
          message.info(`No data available for ${tagName} in the last 24 hours`);
        }
      }
    } catch (error) {
      console.error('Error fetching chart data:', error);
      // Fallback to existing logs if API call fails
      const tagLogs = logs.filter(log => 
        log.device_id === selectedDevice && 
        log.tag_name === tagName
      ).sort((a, b) => moment(a.timestamp).valueOf() - moment(b.timestamp).valueOf());

      const chartDataForTag = tagLogs.map(log => ({
        time: moment(log.timestamp).format('MM-DD HH:mm'),
        value: log.value,
        timestamp: log.timestamp
      }));

      setChartData(chartDataForTag);
      setSelectedTagForChart(tagName);
      
      if (chartDataForTag.length === 0) {
        message.info(`No data available for ${tagName}`);
      }
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
      title: 'Tag',
      dataIndex: 'tag',
      key: 'tag',
      sorter: (a, b) => a.tag.localeCompare(b.tag),
    },
    {
      title: 'Value',
      dataIndex: 'value',
      key: 'value',
      render: (value, record) => (
        <span>
          {value !== 'NA' && typeof value === 'number' ? value.toFixed(3) : value}
          {value !== 'NA' && record.unit && ` ${record.unit}`}
        </span>
      ),
    },
    {
      title: 'Timestamp',
      dataIndex: 'timestamp',
      key: 'timestamp',
      sorter: (a, b) => {
        if (a.timestamp === 'NA' || b.timestamp === 'NA') return 0;
        return moment(a.timestamp).valueOf() - moment(b.timestamp).valueOf();
      },
    },
    {
      title: 'Quality',
      dataIndex: 'quality',
      key: 'quality',
      render: (quality) => (
        <Tag color={quality === 'Good' ? 'success' : quality === 'NA' ? 'default' : 'error'}>
          {quality}
        </Tag>
      ),
    },
    {
      title: 'Visualize',
      key: 'visualize',
      render: (_, record) => (
        <Button
          type="primary"
          size="small"
          icon={<LineChartOutlined />}
          onClick={() => handleVisualize(record.tagName)}
          disabled={record.value === 'NA'}
        >
          Visualize
        </Button>
      ),
    },
  ];

  // Calculate statistics
  const goodQualityData = tableData.filter(row => row.quality === 'Good').length;
  const noDataCount = tableData.filter(row => row.value === 'NA').length;

  return (
    <div>
      <Row gutter={[16, 16]}>
        <Col span={6}>
          <Card>
            <Statistic
              title={selectedDevice ? "Total Tags" : "Total Logs"}
              value={selectedDevice ? tableData.length : logs.length}
              prefix={<DatabaseOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Good Quality"
              value={selectedDevice ? goodQualityData : logs.filter(log => log.quality === 'Good').length}
              valueStyle={{ color: '#3f8600' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title={selectedDevice ? "No Data (24h)" : "Bad Quality"}
              value={selectedDevice ? noDataCount : logs.filter(log => log.quality !== 'Good').length}
              valueStyle={{ color: '#cf1322' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title={selectedDevice ? "With Data (24h)" : "Unique Tags"}
              value={selectedDevice ? (tableData.length - noDataCount) : new Set(logs.map(log => log.tag_name)).size}
            />
          </Card>
        </Col>
      </Row>

      <Card 
        title={selectedTagForChart ? `24-Hour History: ${selectedTagForChart}` : "Select a tag to visualize 24-hour history"} 
        style={{ marginTop: 16 }}
      >
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="time" />
              <YAxis />
              <Tooltip 
                labelFormatter={(value) => `Time: ${value}`}
                formatter={(value) => [value, 'Value']}
              />
              <Line 
                type="monotone" 
                dataKey="value" 
                stroke="#1890ff" 
                strokeWidth={2}
                dot={true}
                dotSize={4}
              />
            </LineChart>
          </ResponsiveContainer>
        ) : (
          <div style={{ textAlign: 'center', padding: '60px 0', color: '#999' }}>
            {selectedTagForChart ? 
              `No data available for ${selectedTagForChart} in the last 24 hours` : 
              'Click "Visualize" button on any tag to view its 24-hour history'
            }
          </div>
        )}
      </Card>

      <Card
        title={selectedDevice ? "Device Tag Status (Latest 24h Values)" : "Select a device to view tag status"}
        style={{ marginTop: 16 }}
        extra={
          <Space>
            <Select
              placeholder="Select device to view tags"
              style={{ width: 250 }}
              value={selectedDevice}
              onChange={handleDeviceChange}
              allowClear
            >
              {devices.map(device => (
                <Option key={device.id} value={device.id}>
                  {device.name}
                </Option>
              ))}
            </Select>
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
        {selectedDevice && (
          <div style={{ marginBottom: 16, padding: '8px 16px', backgroundColor: '#f6ffed', border: '1px solid #b7eb8f', borderRadius: '6px' }}>
            <span style={{ color: '#52c41a', fontWeight: 'bold' }}>ℹ️ Showing latest values for each tag within the last 24 hours. Click "Visualize" to see 24-hour history for any tag.</span>
          </div>
        )}
        <Table
          dataSource={selectedDevice ? tableData : []}
          columns={columns}
          loading={loading}
          rowKey="key"
          pagination={{
            ...pagination,
            total: tableData.length,
            showSizeChanger: true,
            showQuickJumper: true,
            showTotal: (total, range) =>
              `${range[0]}-${range[1]} of ${total} tags`,
          }}
          onChange={handleTableChange}
          size="small"
          locale={{
            emptyText: selectedDevice ? 'No tags found for this device' : 'Please select a device to view its tags'
          }}
        />
      </Card>
    </div>
  );
};

export default DataLogs;
