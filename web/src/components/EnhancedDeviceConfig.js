import React, { useState, useEffect } from 'react';
import {
  Card,
  Form,
  Input,
  Select,
  Switch,
  Button,
  Table,
  Modal,
  Space,
  message,
  Row,
  Col,
  Divider,
  Tag,
  InputNumber,
  Tooltip,
  Typography,
} from 'antd';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  InfoCircleOutlined,
  SettingOutlined,
  SearchOutlined,
} from '@ant-design/icons';
import axios from 'axios';
import DeviceModelBrowser from './DeviceModelBrowser';

const { Option } = Select;
const { Text } = Typography;

const EnhancedDeviceConfig = () => {
  const [devices, setDevices] = useState([]);
  const [deviceModels, setDeviceModels] = useState([]);
  const [scheduleGroups, setScheduleGroups] = useState([]);
  const [selectedModel, setSelectedModel] = useState(null);
  const [tagTemplates, setTagTemplates] = useState([]);
  const [loading, setLoading] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [editingDevice, setEditingDevice] = useState(null);
  const [form] = Form.useForm();
  const [deviceTags, setDeviceTags] = useState([]);
  const [modelBrowserVisible, setModelBrowserVisible] = useState(false);

  // Fetch device models on component mount
  useEffect(() => {
    fetchDeviceModels();
    fetchScheduleGroups();
    fetchDevices();
  }, []);

  const fetchDeviceModels = async () => {
    try {
      console.log('EnhancedDeviceConfig: Fetching device models...');
      const response = await axios.get('/api/device-models');
      if (response.data.success) {
        const models = response.data.data;
        console.log(`EnhancedDeviceConfig: Loaded ${models.length} device models:`, models);
        
        // Check for the specific model mentioned
        const sungrowModel = models.find(m => 
          m.name?.toLowerCase().includes('sungrow') || 
          m.manufacturer?.toLowerCase().includes('huawei')
        );
        if (sungrowModel) {
          console.log('EnhancedDeviceConfig: Found Sungrow/Huawei model:', sungrowModel);
        } else {
          console.log('EnhancedDeviceConfig: Sungrow/Huawei model not found in response');
        }
        
        setDeviceModels(models);
      }
    } catch (error) {
      console.error('EnhancedDeviceConfig: Failed to fetch device models:', error);
      message.error('Failed to fetch device models');
    }
  };

  const fetchScheduleGroups = async () => {
    try {
      const response = await axios.get('/api/schedule-groups');
      if (response.data.success) {
        setScheduleGroups(response.data.data.filter(group => group.enabled));
      }
    } catch (error) {
      message.error('Failed to fetch schedule groups');
    }
  };

  const fetchDevices = async () => {
    try {
      setLoading(true);
      const response = await axios.get('/api/devices-enhanced');
      if (response.data.success) {
        setDevices(response.data.data);
      }
    } catch (error) {
      message.error('Failed to fetch devices');
    } finally {
      setLoading(false);
    }
  };

  const fetchTagTemplates = async (modelId) => {
    if (!modelId || modelId === 'custom') {
      setTagTemplates([]);
      return;
    }

    try {
      // Find the model name from the modelId
      const model = deviceModels.find(m => m.id === modelId);
      if (!model) {
        console.error('Model not found for ID:', modelId);
        setTagTemplates([]);
        return;
      }

      console.log('Fetching tag templates for model:', model.name);
      const response = await axios.get(`/api/modbus-tcp-tag-registers?device_model=${encodeURIComponent(model.name)}`);
      if (response.data.success) {
        // Transform the data to match the expected format
        const transformedData = response.data.data.map(item => ({
          id: item.id,
          name: item.data_label,
          address: item.address,
          data_type: item.modbus_type,
          description: `${item.ava_type}${item.mppt ? ` - MPPT ${item.mppt}` : ''}${item.input ? ` - Input ${item.input}` : ''} (${item.device_model})`,
          scaling_multiplier: item.divider,
          scaling_offset: 0,
          unit: item.register_type,
          read_only: item.register_type === 'input',
          // Keep original fields for compatibility
          data_label: item.data_label,
          modbus_type: item.modbus_type,
          ava_type: item.ava_type,
          mppt: item.mppt,
          input: item.input,
          divider: item.divider,
          register_type: item.register_type,
          device_model: item.device_model
        }));
        
        console.log(`Loaded ${transformedData.length} tag templates`);
        setTagTemplates(transformedData);
      }
    } catch (error) {
      console.error('Failed to fetch tag templates:', error);
      message.error('Failed to fetch tag templates');
    }
  };

  const handleModelChange = (modelId) => {
    setSelectedModel(modelId);
    
    // Don't automatically fetch tag templates here
    // Tags will only load when the "Enabled" toggle is switched on
    
    // Clear existing tags if switching to custom or clearing selection
    if (!modelId || modelId === 'custom') {
      setTagTemplates([]);
      setDeviceTags([]);
    }
    
    console.log('Model changed to:', modelId, '- tags will load when device is enabled');
  };

  // Auto-load tag templates when enabled is toggled to true
  const handleFormValuesChange = async (changedValues, allValues) => {
    // If enabled is being turned on and we have a model selected
    if (changedValues.enabled === true && allValues.model_id && allValues.model_id !== 'custom') {
      console.log('Auto-loading tag templates for model:', allValues.model_id);
      
      // Check if we already have tag templates loaded for this model
      if (tagTemplates.length === 0 || selectedModel !== allValues.model_id) {
        setSelectedModel(allValues.model_id);
        await fetchTagTemplates(allValues.model_id);
      }
    }
  };

  // Update device tags when tag templates change
  useEffect(() => {
    if (tagTemplates.length > 0) {
      const defaultScheduleGroup = scheduleGroups.find(group => group.id === 'medium_freq') || scheduleGroups[0];
      const newTags = tagTemplates.map(template => ({
        name: template.name,
        address: template.address,
        data_type: template.data_type,
        description: template.description,
        scaling_multiplier: template.scaling_multiplier,
        scaling_offset: template.scaling_offset,
        unit: template.unit,
        read_only: template.read_only,
        enabled: true,
        schedule_group_id: defaultScheduleGroup?.id || null,
      }));
      setDeviceTags(newTags);
    }
  }, [tagTemplates, scheduleGroups]);

  const showAddModal = () => {
    setEditingDevice(null);
    setSelectedModel(null);
    setTagTemplates([]);
    setDeviceTags([]);
    form.resetFields();
    setModalVisible(true);
  };

  const showEditModal = (device) => {
    setEditingDevice(device);
    setSelectedModel(device.device.model_id);
    setDeviceTags(device.tags);
    
    // Parse protocol config
    let protocolConfig = {};
    try {
      protocolConfig = JSON.parse(device.device.protocol_config);
    } catch (e) {
      console.error('Failed to parse protocol config:', e);
    }

    form.setFieldsValue({
      id: device.device.id,
      name: device.device.name,
      model_id: device.device.model_id,
      enabled: device.device.enabled,
      polling_interval_ms: device.device.polling_interval_ms,
      timeout_ms: device.device.timeout_ms,
      retry_count: device.device.retry_count,
      protocol_type: protocolConfig.type || 'modbus_tcp',
      host: protocolConfig.host || '',
      port: protocolConfig.port || (protocolConfig.type === 'iec104' ? 2404 : 502),
      slave_id: protocolConfig.slave_id || 1,
      baud_rate: protocolConfig.baud_rate || 9600,
      common_address: protocolConfig.common_address || 1,
    });

    // Don't automatically load tag templates when editing
    // The existing device tags are already loaded in setDeviceTags(device.tags)
    // If user wants to reload tag templates, they can toggle "Enabled" off and on
    console.log('Editing device - using existing tags, templates will reload if enabled is toggled');
    
    setModalVisible(true);
  };

  const handleSubmit = async (values) => {
    try {
      setLoading(true);

      // Build protocol config based on protocol type
      const protocolConfig = {
        type: values.protocol_type,
      };

      if (values.protocol_type === 'modbus_tcp') {
        protocolConfig.host = values.host;
        protocolConfig.port = values.port;
        protocolConfig.slave_id = values.slave_id;
      } else if (values.protocol_type === 'modbus_rtu') {
        protocolConfig.port = values.port; // Serial port path
        protocolConfig.baud_rate = values.baud_rate;
        protocolConfig.slave_id = values.slave_id;
      } else if (values.protocol_type === 'iec104') {
        protocolConfig.host = values.host;
        protocolConfig.port = values.port;
        protocolConfig.common_address = values.common_address;
      }

      const deviceData = {
        id: values.id,
        name: values.name,
        model_id: values.model_id || null,
        enabled: values.enabled || false,
        polling_interval_ms: values.polling_interval_ms || 1000,
        timeout_ms: values.timeout_ms || 5000,
        retry_count: values.retry_count || 3,
        protocol_config: protocolConfig,
        tags: deviceTags,
      };

      let response;
      if (editingDevice) {
        // Update existing device
        response = await axios.put(`/api/devices-enhanced/${values.id}`, deviceData);
      } else {
        // Create new device
        response = await axios.post('/api/devices-enhanced', deviceData);
      }

      if (response.data.success) {
        message.success(editingDevice ? 'Device updated successfully' : 'Device created successfully');
        setModalVisible(false);
        fetchDevices();
      } else {
        message.error(editingDevice ? 'Failed to update device' : 'Failed to create device');
      }
    } catch (error) {
      message.error(editingDevice ? 'Failed to update device' : 'Failed to create device');
      console.error('Error saving device:', error);
    } finally {
      setLoading(false);
    }
  };

  const addCustomTag = () => {
    const defaultScheduleGroup = scheduleGroups.find(group => group.id === 'medium_freq') || scheduleGroups[0];
    const newTag = {
      name: '',
      address: 1,
      data_type: 'holding_register',
      description: '',
      scaling_multiplier: 1.0,
      scaling_offset: 0.0,
      unit: '',
      read_only: false,
      enabled: true,
      schedule_group_id: defaultScheduleGroup?.id || null,
    };
    setDeviceTags([...deviceTags, newTag]);
  };

  const updateTag = (index, field, value) => {
    const updatedTags = [...deviceTags];
    updatedTags[index] = { ...updatedTags[index], [field]: value };
    setDeviceTags(updatedTags);
  };

  const removeTag = (index) => {
    const updatedTags = deviceTags.filter((_, i) => i !== index);
    setDeviceTags(updatedTags);
  };

  const getProtocolTypeColor = (type) => {
    const colors = {
      modbus_tcp: 'blue',
      modbus_rtu: 'green',
      iec104: 'orange',
    };
    return colors[type] || 'default';
  };

  const formatInterval = (intervalMs) => {
    if (intervalMs < 1000) {
      return `${intervalMs}ms`;
    } else if (intervalMs < 60000) {
      return `${intervalMs / 1000}s`;
    } else {
      return `${intervalMs / 60000}min`;
    }
  };

  const getScheduleGroupColor = (intervalMs) => {
    if (intervalMs <= 100) return 'red';
    if (intervalMs <= 1000) return 'orange';
    if (intervalMs <= 5000) return 'blue';
    return 'green';
  };

  const deviceColumns = [
    {
      title: 'Device Name',
      dataIndex: ['device', 'name'],
      key: 'name',
    },
    {
      title: 'Model',
      dataIndex: ['device', 'model_id'],
      key: 'model_id',
      render: (modelId) => {
        if (!modelId) return <Tag color="default">Custom</Tag>;
        const model = deviceModels.find(m => m.id === modelId);
        return model ? (
          <Tooltip title={model.description}>
            <Tag color="blue">{model.name}</Tag>
          </Tooltip>
        ) : <Tag color="default">Unknown</Tag>;
      },
    },
    {
      title: 'Protocol',
      dataIndex: ['device', 'protocol_config'],
      key: 'protocol',
      render: (config) => {
        try {
          const protocolConfig = JSON.parse(config);
          return <Tag color={getProtocolTypeColor(protocolConfig.type)}>{protocolConfig.type?.toUpperCase()}</Tag>;
        } catch (e) {
          return <Tag color="default">Unknown</Tag>;
        }
      },
    },
    {
      title: 'Status',
      dataIndex: ['device', 'enabled'],
      key: 'enabled',
      render: (enabled) => (
        <Tag color={enabled ? 'green' : 'red'}>
          {enabled ? 'Enabled' : 'Disabled'}
        </Tag>
      ),
    },
    {
      title: 'Tags',
      dataIndex: 'tags',
      key: 'tags',
      render: (tags) => {
        const scaledTags = tags?.filter(tag => 
          tag.scaling_multiplier !== 1.0 || tag.scaling_offset !== 0.0
        ).length || 0;
        
        return (
          <Space>
            <Text>{tags?.length || 0} tags</Text>
            {scaledTags > 0 && (
              <Tooltip title={`${scaledTags} tag(s) have custom scaling`}>
                <Tag size="small" color="orange">
                  {scaledTags} scaled
                </Tag>
              </Tooltip>
            )}
          </Space>
        );
      },
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            type="text"
            icon={<EditOutlined />}
            onClick={() => showEditModal(record)}
          />
          <Button
            type="text"
            danger
            icon={<DeleteOutlined />}
            onClick={() => console.log('Delete:', record)}
          />
        </Space>
      ),
    },
  ];

  const tagColumns = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (value, record, index) => (
        <Input
          value={value}
          onChange={(e) => updateTag(index, 'name', e.target.value)}
          placeholder="Tag name"
        />
      ),
    },
    {
      title: 'Address',
      dataIndex: 'address',
      key: 'address',
      width: 100,
      render: (value, record, index) => (
        <InputNumber
          value={value}
          onChange={(val) => updateTag(index, 'address', val)}
          min={1}
          max={65535}
        />
      ),
    },
    {
      title: 'Data Type',
      dataIndex: 'data_type',
      key: 'data_type',
      render: (value, record, index) => (
        <Select
          value={value}
          onChange={(val) => updateTag(index, 'data_type', val)}
          style={{ width: '100%' }}
        >
          <Option value="coil">Coil</Option>
          <Option value="discrete_input">Discrete Input</Option>
          <Option value="holding_register">Holding Register</Option>
          <Option value="input_register">Input Register</Option>
          <Option value="uint16">UInt16</Option>
          <Option value="int16">Int16</Option>
          <Option value="uint32">UInt32</Option>
          <Option value="int32">Int32</Option>
          <Option value="float32">Float32</Option>
        </Select>
      ),
    },
    {
      title: 'Unit',
      dataIndex: 'unit',
      key: 'unit',
      width: 80,
      render: (value, record, index) => (
        <Input
          value={value || ''}
          onChange={(e) => updateTag(index, 'unit', e.target.value)}
          placeholder="Unit"
        />
      ),
    },
    {
      title: (
        <Tooltip title="Multiplier applied to raw value: scaled_value = (raw_value * multiplier) + offset">
          Scale Multiplier
        </Tooltip>
      ),
      dataIndex: 'scaling_multiplier',
      key: 'scaling_multiplier',
      width: 120,
      render: (value, record, index) => (
        <InputNumber
          value={value}
          onChange={(val) => updateTag(index, 'scaling_multiplier', val || 1.0)}
          min={0.001}
          max={1000}
          step={0.001}
          precision={3}
          placeholder="1.0"
          style={{ width: '100%' }}
        />
      ),
    },
    {
      title: (
        <Tooltip title="Offset added to scaled value: scaled_value = (raw_value * multiplier) + offset">
          Scale Offset
        </Tooltip>
      ),
      dataIndex: 'scaling_offset',
      key: 'scaling_offset',
      width: 120,
      render: (value, record, index) => (
        <InputNumber
          value={value}
          onChange={(val) => updateTag(index, 'scaling_offset', val || 0.0)}
          min={-1000000}
          max={1000000}
          step={0.001}
          precision={3}
          placeholder="0.0"
          style={{ width: '100%' }}
        />
      ),
    },
    {
      title: 'Schedule Group',
      dataIndex: 'schedule_group_id',
      key: 'schedule_group_id',
      width: 150,
      render: (value, record, index) => (
        <Select
          value={value}
          onChange={(val) => updateTag(index, 'schedule_group_id', val)}
          style={{ width: '100%' }}
          placeholder="Select schedule"
          allowClear
        >
          {scheduleGroups.map(group => (
            <Option key={group.id} value={group.id}>
              <Space>
                <span>{group.name}</span>
                <Tag size="small" color={getScheduleGroupColor(group.polling_interval_ms)}>
                  {formatInterval(group.polling_interval_ms)}
                </Tag>
              </Space>
            </Option>
          ))}
        </Select>
      ),
    },
    {
      title: 'Enabled',
      dataIndex: 'enabled',
      key: 'enabled',
      width: 80,
      render: (value, record, index) => (
        <Switch
          checked={value}
          onChange={(checked) => updateTag(index, 'enabled', checked)}
        />
      ),
    },
    {
      title: 'Action',
      key: 'action',
      width: 80,
      render: (_, record, index) => (
        <Button
          type="text"
          danger
          icon={<DeleteOutlined />}
          onClick={() => removeTag(index)}
        />
      ),
    },
  ];

  return (
    <div>
      <Row gutter={[16, 16]}>
        <Col span={24}>
          <Card
            title={
              <Space>
                <SettingOutlined />
                Enhanced Device Configuration
              </Space>
            }
            extra={
              <Button type="primary" icon={<PlusOutlined />} onClick={showAddModal}>
                Add Device
              </Button>
            }
          >
            <Table
              columns={deviceColumns}
              dataSource={devices}
              loading={loading}
              rowKey={(record) => record.device.id}
              pagination={false}
            />
          </Card>
        </Col>
      </Row>

      <Modal
        title={editingDevice ? 'Edit Device' : 'Add Device'}
        open={modalVisible}
        onCancel={() => setModalVisible(false)}
        width={1200}
        footer={null}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
          onValuesChange={handleFormValuesChange}
          initialValues={{
            enabled: false,
            polling_interval_ms: 1000,
            timeout_ms: 5000,
            retry_count: 3,
            protocol_type: 'modbus_tcp',
            host: '192.168.1.100',
            port: 502,
            slave_id: 1,
            baud_rate: 9600,
            common_address: 1,
          }}
        >
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                label="Device ID"
                name="id"
                rules={[{ required: true, message: 'Please enter device ID' }]}
              >
                <Input placeholder="unique_device_id" disabled={!!editingDevice} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                label="Device Name"
                name="name"
                rules={[{ required: true, message: 'Please enter device name' }]}
              >
                <Input placeholder="My Device" />
              </Form.Item>
            </Col>
          </Row>

          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                label={
                  <Space>
                    Device Model
                    <Tooltip title="Select a device model to auto-populate tags">
                      <InfoCircleOutlined />
                    </Tooltip>
                  </Space>
                }
                name="model_id"
              >
                <Select
                  placeholder="Select device model"
                  onChange={handleModelChange}
                  allowClear
                  dropdownRender={(menu) => (
                    <>
                      {menu}
                      <Divider style={{ margin: '8px 0' }} />
                      <Button
                        type="text"
                        icon={<SearchOutlined />}
                        onClick={() => setModelBrowserVisible(true)}
                        style={{ width: '100%' }}
                      >
                        Browse All Models
                      </Button>
                    </>
                  )}
                >
                  <Option value="custom">Custom Device (No predefined tags)</Option>
                  {deviceModels.map(model => (
                    <Option key={model.id} value={model.id}>
                      {model.name} - {model.manufacturer}
                    </Option>
                  ))}
                </Select>
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item label="Enabled" name="enabled" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
          </Row>

          <Divider>Protocol Configuration</Divider>

          <Form.Item
            name="protocol_type"
            label="Protocol Type"
            rules={[{ required: true, message: 'Please select protocol type' }]}
          >
            <Select placeholder="Select protocol type">
              <Option value="modbus_tcp">Modbus TCP</Option>
              <Option value="modbus_rtu">Modbus RTU</Option>
              <Option value="iec104">IEC 104</Option>
            </Select>
          </Form.Item>

          <Form.Item dependencies={['protocol_type']}>
            {({ getFieldValue }) => {
              const protocolType = getFieldValue('protocol_type');
              
              if (protocolType === 'modbus_tcp') {
                return (
                  <Row gutter={16}>
                    <Col span={8}>
                      <Form.Item
                        name="host"
                        label="Host"
                        rules={[{ required: true, message: 'Please enter host' }]}
                      >
                        <Input placeholder="192.168.1.100" />
                      </Form.Item>
                    </Col>
                    <Col span={8}>
                      <Form.Item
                        name="port"
                        label="Port"
                        rules={[{ required: true, message: 'Please enter port' }]}
                      >
                        <InputNumber min={1} max={65535} placeholder="502" style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                    <Col span={8}>
                      <Form.Item
                        name="slave_id"
                        label="Slave ID"
                        rules={[{ required: true, message: 'Please enter slave ID' }]}
                      >
                        <InputNumber min={1} max={255} placeholder="1" style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                  </Row>
                );
              } else if (protocolType === 'modbus_rtu') {
                return (
                  <Row gutter={16}>
                    <Col span={8}>
                      <Form.Item
                        name="port"
                        label="Serial Port"
                        rules={[{ required: true, message: 'Please enter serial port' }]}
                      >
                        <Input placeholder="/dev/ttyUSB0" />
                      </Form.Item>
                    </Col>
                    <Col span={8}>
                      <Form.Item
                        name="baud_rate"
                        label="Baud Rate"
                        rules={[{ required: true, message: 'Please enter baud rate' }]}
                      >
                        <Select placeholder="Select baud rate" style={{ width: '100%' }}>
                          <Option value={9600}>9600</Option>
                          <Option value={19200}>19200</Option>
                          <Option value={38400}>38400</Option>
                          <Option value={57600}>57600</Option>
                          <Option value={115200}>115200</Option>
                        </Select>
                      </Form.Item>
                    </Col>
                    <Col span={8}>
                      <Form.Item
                        name="slave_id"
                        label="Slave ID"
                        rules={[{ required: true, message: 'Please enter slave ID' }]}
                      >
                        <InputNumber min={1} max={255} placeholder="1" style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                  </Row>
                );
              } else if (protocolType === 'iec104') {
                return (
                  <Row gutter={16}>
                    <Col span={8}>
                      <Form.Item
                        name="host"
                        label="Host"
                        rules={[{ required: true, message: 'Please enter host' }]}
                      >
                        <Input placeholder="192.168.1.100" />
                      </Form.Item>
                    </Col>
                    <Col span={8}>
                      <Form.Item
                        name="port"
                        label="Port"
                        rules={[{ required: true, message: 'Please enter port' }]}
                      >
                        <InputNumber min={1} max={65535} placeholder="2404" style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                    <Col span={8}>
                      <Form.Item
                        name="common_address"
                        label="Common Address"
                        rules={[{ required: true, message: 'Please enter common address' }]}
                      >
                        <InputNumber min={1} max={65535} placeholder="1" style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                  </Row>
                );
              }
              return null;
            }}
          </Form.Item>

          <Row gutter={16}>
            <Col span={8}>
              <Form.Item label="Polling Interval (ms)" name="polling_interval_ms">
                <InputNumber min={100} max={60000} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item label="Timeout (ms)" name="timeout_ms">
                <InputNumber min={1000} max={30000} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item label="Retry Count" name="retry_count">
                <InputNumber min={1} max={10} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>

          <Divider>
            <Space>
              Tag Configuration
              <Button
                type="dashed"
                size="small"
                icon={<PlusOutlined />}
                onClick={addCustomTag}
              >
                Add Custom Tag
              </Button>
            </Space>
          </Divider>

          <Table
            columns={tagColumns}
            dataSource={deviceTags}
            pagination={false}
            rowKey={(record, index) => index}
            size="small"
            scroll={{ x: true }}
          />

          <div style={{ marginTop: 16, textAlign: 'right' }}>
            <Space>
              <Button onClick={() => setModalVisible(false)}>Cancel</Button>
              <Button type="primary" htmlType="submit" loading={loading}>
                {editingDevice ? 'Update Device' : 'Create Device'}
              </Button>
            </Space>
          </div>
        </Form>
      </Modal>

      <DeviceModelBrowser
        visible={modelBrowserVisible}
        onClose={() => setModelBrowserVisible(false)}
        onSelectModel={(model) => {
          console.log('Model selected in EnhancedDeviceConfig:', model);
          
          // Check if the model already exists in deviceModels
          const existingModelIndex = deviceModels.findIndex(m => m.id === model.id);
          
          if (existingModelIndex === -1) {
            // Add the new model to the deviceModels list if it doesn't exist
            setDeviceModels(prev => [...prev, model]);
          } else {
            // Update the existing model with fresh data
            setDeviceModels(prev => prev.map(m => m.id === model.id ? model : m));
          }
          
          form.setFieldValue('model_id', model.id);
          setSelectedModel(model.id);
          
          // Don't automatically load tag templates here
          // Tags will only load when the "Enabled" toggle is switched on
          console.log('Model selected - tags will load when device is enabled');
          
          setModelBrowserVisible(false);
        }}
        onTagTemplatesLoaded={(templates, model) => {
          console.log('Tag templates loaded callback:', templates.length, model.name);
          // Don't automatically load tag templates here - they should only load when device is enabled
          console.log('Tag templates available but not loaded - will load when device is enabled');
        }}
      />
    </div>
  );
};

export default EnhancedDeviceConfig;
