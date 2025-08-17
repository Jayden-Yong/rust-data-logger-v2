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
  Form,
  Upload,
  message,
} from 'antd';
import {
  SearchOutlined,
  InfoCircleOutlined,
  TagsOutlined,
  DeleteOutlined,
  PlusOutlined,
  UploadOutlined,
} from '@ant-design/icons';
import axios from 'axios';

const { Search } = Input;
const { Option } = Select;
const { Title, Text } = Typography;
const { TextArea } = Input;

const DeviceModelBrowser = ({ onSelectModel, visible, onClose }) => {
  const [deviceModels, setDeviceModels] = useState([]);
  const [filteredModels, setFilteredModels] = useState([]);
  const [loading, setLoading] = useState(false);
  const [selectedModel, setSelectedModel] = useState(null);
  const [tagTemplates, setTagTemplates] = useState([]);
  const [searchText, setSearchText] = useState('');
  const [protocolFilter, setProtocolFilter] = useState('all');
  const [addModelVisible, setAddModelVisible] = useState(false);
  const [addModelLoading, setAddModelLoading] = useState(false);
  const [csvFile, setCsvFile] = useState(null);
  const [form] = Form.useForm();

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

  const handleAddModel = () => {
    setAddModelVisible(true);
  };

  const handleAddModelCancel = () => {
    setAddModelVisible(false);
    form.resetFields();
    setCsvFile(null);
  };

  const handleCsvUpload = ({ file, fileList }) => {
    if (fileList.length > 0) {
      setCsvFile(file);
    } else {
      setCsvFile(null);
    }
    return false; // Prevent auto upload
  };

  const handleAddModelSubmit = async (values) => {
    try {
      setAddModelLoading(true);
      
      const formData = new FormData();
      formData.append('name', values.name);
      formData.append('manufacturer', values.manufacturer || '');
      formData.append('protocol_type', values.protocol_type);
      formData.append('description', values.description || '');
      
      if (csvFile) {
        formData.append('csv_file', csvFile);
      }

      const response = await axios.post('/api/device-models', formData, {
        headers: {
          'Content-Type': 'multipart/form-data',
        },
      });

      if (response.data.success) {
        message.success('Device model created successfully');
        setAddModelVisible(false);
        form.resetFields();
        setCsvFile(null);
        fetchDeviceModels(); // Refresh the list
      } else {
        throw new Error(response.data.error || 'Failed to create device model');
      }
    } catch (error) {
      console.error('Error creating device model:', error);
      message.error(`Failed to create device model: ${error.response?.data?.error || error.message}`);
    } finally {
      setAddModelLoading(false);
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

  const handleDeleteModel = async (modelId) => {
    try {
      setLoading(true);
      const response = await axios.post(`/api/device-models/${modelId}/delete`);
      
      if (response.data.success) {
        Modal.success({
          title: 'Success',
          content: 'Device model deleted successfully',
        });
        // Remove the deleted model from the local state
        setDeviceModels(prev => prev.filter(model => model.id !== modelId));
        setFilteredModels(prev => prev.filter(model => model.id !== modelId));
        
        // Clear selected model if it was the deleted one
        if (selectedModel && selectedModel.id === modelId) {
          setSelectedModel(null);
          setTagTemplates([]);
        }
      } else {
        throw new Error(response.data.error || 'Failed to delete device model');
      }
    } catch (error) {
      console.error('Error deleting device model:', error);
      Modal.error({
        title: 'Error',
        content: `Failed to delete device model: ${error.response?.data?.error || error.message}`,
      });
    } finally {
      setLoading(false);
    }
  };

  const showDeleteConfirm = (model) => {
    Modal.confirm({
      title: 'Delete Device Model',
      content: `Are you sure you want to delete "${model.name}"? This action cannot be undone and will also delete all associated tag templates.`,
      okText: 'Delete',
      okType: 'danger',
      cancelText: 'Cancel',
      onOk() {
        handleDeleteModel(model.id);
      },
    });
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
          <Button
            type="text"
            danger
            icon={<DeleteOutlined />}
            onClick={() => showDeleteConfirm(record)}
          >
            Delete
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
    <>
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
        {/* Filters and Add Button */}
        <Col span={24}>
          <Space size="middle" style={{ width: '100%', justifyContent: 'space-between' }}>
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
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={handleAddModel}
            >
              Add New Model
            </Button>
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

    {/* Add Model Modal */}
    <Modal
      title="Add New Device Model"
      open={addModelVisible}
      onCancel={handleAddModelCancel}
      confirmLoading={addModelLoading}
      onOk={() => form.submit()}
      width={600}
    >
      <Form
        form={form}
        layout="vertical"
        onFinish={handleAddModelSubmit}
      >
        <Form.Item
          name="name"
          label="Device Model Name"
          rules={[
            { required: true, message: 'Please enter device model name' },
            { min: 2, message: 'Name must be at least 2 characters' },
          ]}
        >
          <Input placeholder="e.g., PowerMeter Pro 3000" />
        </Form.Item>

        <Form.Item
          name="manufacturer"
          label="Manufacturer"
          rules={[{ required: true, message: 'Please enter manufacturer name' }]}
        >
          <Input placeholder="e.g., Schneider Electric" />
        </Form.Item>

        <Form.Item
          name="protocol_type"
          label="Protocol Type"
          rules={[{ required: true, message: 'Please select a protocol' }]}
        >
          <Select placeholder="Select communication protocol">
            <Option value="modbus_tcp">Modbus TCP</Option>
            <Option value="modbus_rtu">Modbus RTU</Option>
            <Option value="iec104">IEC 104</Option>
          </Select>
        </Form.Item>

        <Form.Item
          name="description"
          label="Description (Optional)"
        >
          <TextArea
            placeholder="Brief description of the device model"
            rows={3}
          />
        </Form.Item>

        <Form.Item
          label="CSV Tag Template (Optional)"
          extra="Upload a CSV file with tag definitions. Format: name,address,data_type,unit,description"
        >
          <Upload
            accept=".csv"
            beforeUpload={() => false}
            onChange={handleCsvUpload}
            fileList={csvFile ? [csvFile] : []}
            maxCount={1}
          >
            <Button icon={<UploadOutlined />}>Select CSV File</Button>
          </Upload>
        </Form.Item>
      </Form>
    </Modal>
  </>
);
};

export default DeviceModelBrowser;
