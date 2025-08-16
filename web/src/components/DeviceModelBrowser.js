import React, { useState, useEffect } from 'react';
import {
  Card,
  Table,
  Tag,
  Space,
  Input,
  Select,
  Button,
  Modal,
  Descriptions,
  List,
  Typography,
  Row,
  Col,
  Tooltip,
} from 'antd';
import {
  SearchOutlined,
  InfoCircleOutlined,
  TagsOutlined,
} from '@ant-design/icons';
import axios from 'axios';

const { Search } = Input;
const { Option } = Select;
const { Title, Text } = Typography;

const DeviceModelBrowser = ({ onSelectModel, visible, onClose }) => {
  const [deviceModels, setDeviceModels] = useState([]);
  const [filteredModels, setFilteredModels] = useState([]);
  const [loading, setLoading] = useState(false);
  const [selectedModel, setSelectedModel] = useState(null);
  const [tagTemplates, setTagTemplates] = useState([]);
  const [searchText, setSearchText] = useState('');
  const [protocolFilter, setProtocolFilter] = useState('all');

  useEffect(() => {
    if (visible) {
      fetchDeviceModels();
    }
  }, [visible]);

  useEffect(() => {
    filterModels();
  }, [deviceModels, searchText, protocolFilter]);

  const fetchDeviceModels = async () => {
    try {
      setLoading(true);
      const response = await axios.get('/api/device-models');
      if (response.data.success) {
        setDeviceModels(response.data.data);
      }
    } catch (error) {
      console.error('Failed to fetch device models:', error);
    } finally {
      setLoading(false);
    }
  };

  const fetchTagTemplates = async (modelId) => {
    try {
      const response = await axios.get(`/api/device-models/${modelId}/tags`);
      if (response.data.success) {
        setTagTemplates(response.data.data);
      }
    } catch (error) {
      console.error('Failed to fetch tag templates:', error);
      setTagTemplates([]);
    }
  };

  const filterModels = () => {
    let filtered = deviceModels;

    // Filter by search text
    if (searchText) {
      filtered = filtered.filter(model =>
        model.name.toLowerCase().includes(searchText.toLowerCase()) ||
        (model.manufacturer && model.manufacturer.toLowerCase().includes(searchText.toLowerCase())) ||
        (model.description && model.description.toLowerCase().includes(searchText.toLowerCase()))
      );
    }

    // Filter by protocol
    if (protocolFilter !== 'all') {
      filtered = filtered.filter(model => model.protocol_type === protocolFilter);
    }

    setFilteredModels(filtered);
  };

  const handleModelSelect = (model) => {
    setSelectedModel(model);
    fetchTagTemplates(model.id);
  };

  const handleSelectAndClose = () => {
    if (selectedModel && onSelectModel) {
      onSelectModel(selectedModel);
    }
    onClose();
  };

  const getProtocolColor = (protocol) => {
    const colors = {
      modbus_tcp: 'blue',
      modbus_rtu: 'green',
      iec104: 'orange',
      any: 'default',
    };
    return colors[protocol] || 'default';
  };

  const columns = [
    {
      title: 'Model Name',
      dataIndex: 'name',
      key: 'name',
      render: (text, record) => (
        <Space direction="vertical" size="small">
          <Text strong>{text}</Text>
          {record.description && (
            <Text type="secondary" style={{ fontSize: '12px' }}>
              {record.description}
            </Text>
          )}
        </Space>
      ),
    },
    {
      title: 'Manufacturer',
      dataIndex: 'manufacturer',
      key: 'manufacturer',
      render: (text) => text || 'Various',
    },
    {
      title: 'Protocol',
      dataIndex: 'protocol_type',
      key: 'protocol_type',
      render: (protocol) => (
        <Tag color={getProtocolColor(protocol)}>
          {protocol?.toUpperCase() || 'ANY'}
        </Tag>
      ),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            type="text"
            icon={<InfoCircleOutlined />}
            onClick={() => handleModelSelect(record)}
          >
            View Details
          </Button>
        </Space>
      ),
    },
  ];

  const tagColumns = [
    {
      title: 'Tag Name',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: 'Address',
      dataIndex: 'address',
      key: 'address',
      width: 80,
    },
    {
      title: 'Data Type',
      dataIndex: 'data_type',
      key: 'data_type',
      render: (type) => <Tag>{type}</Tag>,
    },
    {
      title: 'Unit',
      dataIndex: 'unit',
      key: 'unit',
      width: 80,
      render: (unit) => unit || '-',
    },
    {
      title: 'Description',
      dataIndex: 'description',
      key: 'description',
      render: (desc) => desc || '-',
    },
  ];

  return (
    <Modal
      title="Device Model Browser"
      open={visible}
      onCancel={onClose}
      width={1400}
      footer={
        selectedModel ? (
          <Space>
            <Button onClick={onClose}>Cancel</Button>
            <Button type="primary" onClick={handleSelectAndClose}>
              Select {selectedModel.name}
            </Button>
          </Space>
        ) : (
          <Button onClick={onClose}>Close</Button>
        )
      }
    >
      <Row gutter={[16, 16]}>
        {/* Filters */}
        <Col span={24}>
          <Space size="middle">
            <Search
              placeholder="Search models..."
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
              style={{ width: 300 }}
              prefix={<SearchOutlined />}
            />
            <Select
              value={protocolFilter}
              onChange={setProtocolFilter}
              style={{ width: 150 }}
            >
              <Option value="all">All Protocols</Option>
              <Option value="modbus_tcp">Modbus TCP</Option>
              <Option value="modbus_rtu">Modbus RTU</Option>
              <Option value="iec104">IEC 104</Option>
              <Option value="any">Generic</Option>
            </Select>
          </Space>
        </Col>

        {/* Model List */}
        <Col span={selectedModel ? 12 : 24}>
          <Card title="Available Device Models" size="small">
            <Table
              columns={columns}
              dataSource={filteredModels}
              loading={loading}
              rowKey="id"
              size="small"
              pagination={false}
              scroll={{ y: 400 }}
              rowSelection={{
                type: 'radio',
                selectedRowKeys: selectedModel ? [selectedModel.id] : [],
                onSelect: (record) => handleModelSelect(record),
              }}
            />
          </Card>
        </Col>

        {/* Model Details */}
        {selectedModel && (
          <Col span={12}>
            <Card
              title={
                <Space>
                  <InfoCircleOutlined />
                  Model Details
                </Space>
              }
              size="small"
            >
              <Descriptions size="small" column={1}>
                <Descriptions.Item label="Name">
                  {selectedModel.name}
                </Descriptions.Item>
                <Descriptions.Item label="Manufacturer">
                  {selectedModel.manufacturer || 'Various'}
                </Descriptions.Item>
                <Descriptions.Item label="Protocol">
                  <Tag color={getProtocolColor(selectedModel.protocol_type)}>
                    {selectedModel.protocol_type?.toUpperCase()}
                  </Tag>
                </Descriptions.Item>
                <Descriptions.Item label="Description">
                  {selectedModel.description || 'No description available'}
                </Descriptions.Item>
              </Descriptions>

              <div style={{ marginTop: 16 }}>
                <Title level={5}>
                  <TagsOutlined /> Predefined Tags ({tagTemplates.length})
                </Title>
                
                {tagTemplates.length > 0 ? (
                  <Table
                    columns={tagColumns}
                    dataSource={tagTemplates}
                    rowKey="id"
                    size="small"
                    pagination={false}
                    scroll={{ y: 200 }}
                  />
                ) : (
                  <Text type="secondary">
                    {selectedModel.id === 'custom' 
                      ? 'Custom devices do not have predefined tags. You can add your own tags after creating the device.'
                      : 'No predefined tags available for this model.'
                    }
                  </Text>
                )}
              </div>
            </Card>
          </Col>
        )}
      </Row>
    </Modal>
  );
};

export default DeviceModelBrowser;
