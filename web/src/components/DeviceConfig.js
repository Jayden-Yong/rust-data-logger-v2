import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Table, 
  Button, 
  Space, 
  Modal, 
  Form, 
  Input, 
  Select, 
  Switch, 
  InputNumber,
  message,
  Divider,
  Tag
} from 'antd';
import { 
  PlusOutlined, 
  EditOutlined, 
  DeleteOutlined, 
  PlayCircleOutlined,
  PauseCircleOutlined 
} from '@ant-design/icons';
import axios from 'axios';

const { Option } = Select;

const DeviceConfig = () => {
  const [devices, setDevices] = useState([]);
  const [loading, setLoading] = useState(true);
  const [modalVisible, setModalVisible] = useState(false);
  const [editingDevice, setEditingDevice] = useState(null);
  const [form] = Form.useForm();

  useEffect(() => {
    fetchDevices();
  }, []);

  const fetchDevices = async () => {
    try {
      const response = await axios.get('/api/devices');
      if (response.data.success) {
        setDevices(response.data.data);
      }
    } catch (error) {
      message.error('Failed to fetch devices');
    } finally {
      setLoading(false);
    }
  };

  const handleAdd = () => {
    setEditingDevice(null);
    setModalVisible(true);
    form.resetFields();
  };

  const handleEdit = (device) => {
    setEditingDevice(device);
    setModalVisible(true);
    form.setFieldsValue(device);
  };

  const handleDelete = async (deviceId) => {
    try {
      const response = await axios.delete(`/api/devices/${deviceId}`);
      if (response.data.success) {
        message.success('Device deleted successfully');
        fetchDevices();
      } else {
        message.error(response.data.error);
      }
    } catch (error) {
      message.error('Failed to delete device');
    }
  };

  const handleSubmit = async (values) => {
    try {
      const url = editingDevice 
        ? `/api/devices/${editingDevice.id}`
        : '/api/devices';
      
      const method = editingDevice ? 'put' : 'post';
      
      const response = await axios[method](url, values);
      
      if (response.data.success) {
        message.success(`Device ${editingDevice ? 'updated' : 'created'} successfully`);
        setModalVisible(false);
        fetchDevices();
      } else {
        message.error(response.data.error);
      }
    } catch (error) {
      message.error(`Failed to ${editingDevice ? 'update' : 'create'} device`);
    }
  };

  const handleDeviceAction = async (deviceId, action) => {
    try {
      const response = await axios.post(`/api/devices/${deviceId}/${action}`);
      if (response.data.success) {
        message.success(`Device ${action}ed successfully`);
        fetchDevices();
      } else {
        message.error(response.data.error);
      }
    } catch (error) {
      message.error(`Failed to ${action} device`);
    }
  };

  const columns = [
    {
      title: 'Device ID',
      dataIndex: 'id',
      key: 'id',
    },
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: 'Protocol',
      dataIndex: 'protocol',
      key: 'protocol',
      render: (protocol) => {
        const type = protocol.type;
        const color = type === 'modbus_tcp' ? 'blue' : 
                    type === 'modbus_rtu' ? 'green' : 'orange';
        return <Tag color={color}>{type.toUpperCase()}</Tag>;
      },
    },
    {
      title: 'Enabled',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled) => (
        <Tag color={enabled ? 'success' : 'default'}>
          {enabled ? 'Yes' : 'No'}
        </Tag>
      ),
    },
    {
      title: 'Polling Interval',
      dataIndex: 'polling_interval_ms',
      key: 'polling_interval_ms',
      render: (interval) => `${interval}ms`,
    },
    {
      title: 'Tags',
      dataIndex: 'tags',
      key: 'tags',
      render: (tags) => tags.length,
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            icon={<EditOutlined />}
            onClick={() => handleEdit(record)}
            size="small"
          />
          <Button
            icon={<DeleteOutlined />}
            onClick={() => handleDelete(record.id)}
            size="small"
            danger
          />
          <Button
            icon={record.enabled ? <PauseCircleOutlined /> : <PlayCircleOutlined />}
            onClick={() => handleDeviceAction(record.id, record.enabled ? 'stop' : 'start')}
            size="small"
            type={record.enabled ? 'default' : 'primary'}
          />
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Card
        title="Device Configuration"
        extra={
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>
            Add Device
          </Button>
        }
      >
        <Table
          dataSource={devices}
          columns={columns}
          loading={loading}
          rowKey="id"
        />
      </Card>

      <Modal
        title={editingDevice ? 'Edit Device' : 'Add Device'}
        open={modalVisible}
        onCancel={() => setModalVisible(false)}
        onOk={() => form.submit()}
        width={800}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
        >
          <Form.Item
            name="id"
            label="Device ID"
            rules={[{ required: true, message: 'Please enter device ID' }]}
          >
            <Input placeholder="Enter unique device ID" />
          </Form.Item>

          <Form.Item
            name="name"
            label="Device Name"
            rules={[{ required: true, message: 'Please enter device name' }]}
          >
            <Input placeholder="Enter device name" />
          </Form.Item>

          <Form.Item
            name="enabled"
            label="Enabled"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>

          <Divider>Protocol Configuration</Divider>

          <Form.Item
            name={['protocol', 'type']}
            label="Protocol Type"
            rules={[{ required: true, message: 'Please select protocol type' }]}
          >
            <Select placeholder="Select protocol type">
              <Option value="modbus_tcp">Modbus TCP</Option>
              <Option value="modbus_rtu">Modbus RTU</Option>
              <Option value="iec104">IEC 104</Option>
            </Select>
          </Form.Item>

          <Form.Item dependencies={[['protocol', 'type']]}>
            {({ getFieldValue }) => {
              const protocolType = getFieldValue(['protocol', 'type']);
              
              if (protocolType === 'modbus_tcp') {
                return (
                  <>
                    <Form.Item
                      name={['protocol', 'host']}
                      label="Host"
                      rules={[{ required: true, message: 'Please enter host' }]}
                    >
                      <Input placeholder="192.168.1.100" />
                    </Form.Item>
                    <Form.Item
                      name={['protocol', 'port']}
                      label="Port"
                      rules={[{ required: true, message: 'Please enter port' }]}
                    >
                      <InputNumber min={1} max={65535} placeholder="502" />
                    </Form.Item>
                    <Form.Item
                      name={['protocol', 'slave_id']}
                      label="Slave ID"
                      rules={[{ required: true, message: 'Please enter slave ID' }]}
                    >
                      <InputNumber min={1} max={255} placeholder="1" />
                    </Form.Item>
                  </>
                );
              } else if (protocolType === 'modbus_rtu') {
                return (
                  <>
                    <Form.Item
                      name={['protocol', 'port']}
                      label="Serial Port"
                      rules={[{ required: true, message: 'Please enter serial port' }]}
                    >
                      <Input placeholder="/dev/ttyUSB0" />
                    </Form.Item>
                    <Form.Item
                      name={['protocol', 'baud_rate']}
                      label="Baud Rate"
                      rules={[{ required: true, message: 'Please enter baud rate' }]}
                    >
                      <Select placeholder="Select baud rate">
                        <Option value={9600}>9600</Option>
                        <Option value={19200}>19200</Option>
                        <Option value={38400}>38400</Option>
                        <Option value={57600}>57600</Option>
                        <Option value={115200}>115200</Option>
                      </Select>
                    </Form.Item>
                    <Form.Item
                      name={['protocol', 'slave_id']}
                      label="Slave ID"
                      rules={[{ required: true, message: 'Please enter slave ID' }]}
                    >
                      <InputNumber min={1} max={255} placeholder="1" />
                    </Form.Item>
                  </>
                );
              } else if (protocolType === 'iec104') {
                return (
                  <>
                    <Form.Item
                      name={['protocol', 'host']}
                      label="Host"
                      rules={[{ required: true, message: 'Please enter host' }]}
                    >
                      <Input placeholder="192.168.1.100" />
                    </Form.Item>
                    <Form.Item
                      name={['protocol', 'port']}
                      label="Port"
                      rules={[{ required: true, message: 'Please enter port' }]}
                    >
                      <InputNumber min={1} max={65535} placeholder="2404" />
                    </Form.Item>
                    <Form.Item
                      name={['protocol', 'common_address']}
                      label="Common Address"
                      rules={[{ required: true, message: 'Please enter common address' }]}
                    >
                      <InputNumber min={1} max={65535} placeholder="1" />
                    </Form.Item>
                  </>
                );
              }
              return null;
            }}
          </Form.Item>

          <Divider>Timing Configuration</Divider>

          <Form.Item
            name="polling_interval_ms"
            label="Polling Interval (ms)"
            rules={[{ required: true, message: 'Please enter polling interval' }]}
          >
            <InputNumber min={100} placeholder="1000" />
          </Form.Item>

          <Form.Item
            name="timeout_ms"
            label="Timeout (ms)"
            rules={[{ required: true, message: 'Please enter timeout' }]}
          >
            <InputNumber min={100} placeholder="5000" />
          </Form.Item>

          <Form.Item
            name="retry_count"
            label="Retry Count"
            rules={[{ required: true, message: 'Please enter retry count' }]}
          >
            <InputNumber min={0} placeholder="3" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default DeviceConfig;
