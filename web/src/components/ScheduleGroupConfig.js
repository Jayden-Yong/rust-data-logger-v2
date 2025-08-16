import React, { useState, useEffect } from 'react';
import {
  Table,
  Button,
  Modal,
  Form,
  Input,
  InputNumber,
  Switch,
  Space,
  message,
  Card,
  Typography,
  Popconfirm,
  Tag,
} from 'antd';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons';
import axios from 'axios';

const { Title } = Typography;

const ScheduleGroupConfig = () => {
  const [scheduleGroups, setScheduleGroups] = useState([]);
  const [loading, setLoading] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [editingGroup, setEditingGroup] = useState(null);
  const [form] = Form.useForm();

  useEffect(() => {
    fetchScheduleGroups();
  }, []);

  const fetchScheduleGroups = async () => {
    try {
      setLoading(true);
      const response = await axios.get('/api/schedule-groups');
      if (response.data.success) {
        setScheduleGroups(response.data.data);
      }
    } catch (error) {
      message.error('Failed to fetch schedule groups');
      console.error('Error fetching schedule groups:', error);
    } finally {
      setLoading(false);
    }
  };

  const showModal = (group = null) => {
    setEditingGroup(group);
    setModalVisible(true);
    if (group) {
      form.setFieldsValue({
        id: group.id,
        name: group.name,
        polling_interval_ms: group.polling_interval_ms,
        description: group.description,
        enabled: group.enabled,
      });
    } else {
      form.resetFields();
    }
  };

  const handleSubmit = async (values) => {
    try {
      setLoading(true);

      const groupData = {
        id: values.id,
        name: values.name,
        polling_interval_ms: values.polling_interval_ms,
        description: values.description || null,
        enabled: values.enabled || true,
      };

      let response;
      if (editingGroup) {
        // Update existing group
        response = await axios.put(`/api/schedule-groups/${values.id}`, groupData);
      } else {
        // Create new group
        response = await axios.post('/api/schedule-groups', groupData);
      }

      if (response.data.success) {
        message.success(editingGroup ? 'Schedule group updated successfully' : 'Schedule group created successfully');
        setModalVisible(false);
        fetchScheduleGroups();
      } else {
        message.error(editingGroup ? 'Failed to update schedule group' : 'Failed to create schedule group');
      }
    } catch (error) {
      message.error(editingGroup ? 'Failed to update schedule group' : 'Failed to create schedule group');
      console.error('Error saving schedule group:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (groupId) => {
    try {
      const response = await axios.delete(`/api/schedule-groups/${groupId}`);
      if (response.data.success) {
        message.success('Schedule group deleted successfully');
        fetchScheduleGroups();
      } else {
        message.error('Failed to delete schedule group');
      }
    } catch (error) {
      message.error('Failed to delete schedule group');
      console.error('Error deleting schedule group:', error);
    }
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

  const getIntervalColor = (intervalMs) => {
    if (intervalMs <= 100) return 'red';
    if (intervalMs <= 1000) return 'orange';
    if (intervalMs <= 5000) return 'blue';
    return 'green';
  };

  const columns = [
    {
      title: 'ID',
      dataIndex: 'id',
      key: 'id',
      width: 150,
    },
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: 'Polling Interval',
      dataIndex: 'polling_interval_ms',
      key: 'polling_interval_ms',
      render: (interval) => (
        <Tag color={getIntervalColor(interval)} icon={<ClockCircleOutlined />}>
          {formatInterval(interval)}
        </Tag>
      ),
    },
    {
      title: 'Description',
      dataIndex: 'description',
      key: 'description',
      render: (description) => description || <span style={{ color: '#ccc' }}>No description</span>,
    },
    {
      title: 'Status',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled) => (
        <Tag color={enabled ? 'success' : 'default'}>
          {enabled ? 'Enabled' : 'Disabled'}
        </Tag>
      ),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            icon={<EditOutlined />}
            onClick={() => showModal(record)}
            size="small"
          >
            Edit
          </Button>
          <Popconfirm
            title="Are you sure you want to delete this schedule group?"
            onConfirm={() => handleDelete(record.id)}
            okText="Yes"
            cancelText="No"
          >
            <Button
              icon={<DeleteOutlined />}
              danger
              size="small"
            >
              Delete
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: '24px' }}>
      <Card>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
          <Title level={3} style={{ margin: 0 }}>Schedule Groups</Title>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => showModal()}
          >
            Add Schedule Group
          </Button>
        </div>

        <Table
          dataSource={scheduleGroups}
          columns={columns}
          loading={loading}
          rowKey="id"
          pagination={{ pageSize: 10 }}
        />
      </Card>

      <Modal
        title={editingGroup ? 'Edit Schedule Group' : 'Add Schedule Group'}
        open={modalVisible}
        onCancel={() => setModalVisible(false)}
        footer={null}
        width={600}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
          initialValues={{
            enabled: true,
            polling_interval_ms: 1000,
          }}
        >
          <Form.Item
            name="id"
            label="ID"
            rules={[
              { required: true, message: 'Please input the schedule group ID!' },
              { pattern: /^[a-z0-9_]+$/, message: 'ID must contain only lowercase letters, numbers, and underscores' }
            ]}
          >
            <Input placeholder="e.g., high_freq, medium_freq" disabled={editingGroup} />
          </Form.Item>

          <Form.Item
            name="name"
            label="Name"
            rules={[{ required: true, message: 'Please input the schedule group name!' }]}
          >
            <Input placeholder="e.g., High Frequency, Medium Frequency" />
          </Form.Item>

          <Form.Item
            name="polling_interval_ms"
            label="Polling Interval (milliseconds)"
            rules={[
              { required: true, message: 'Please input the polling interval!' },
              { type: 'number', min: 10, message: 'Polling interval must be at least 10ms' }
            ]}
          >
            <InputNumber
              min={10}
              max={3600000}
              placeholder="e.g., 1000"
              style={{ width: '100%' }}
              addonAfter="ms"
            />
          </Form.Item>

          <Form.Item
            name="description"
            label="Description"
          >
            <Input.TextArea 
              rows={3} 
              placeholder="Optional description for this schedule group"
            />
          </Form.Item>

          <Form.Item
            name="enabled"
            label="Enabled"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>

          <Form.Item style={{ marginBottom: 0, textAlign: 'right' }}>
            <Space>
              <Button onClick={() => setModalVisible(false)}>
                Cancel
              </Button>
              <Button type="primary" htmlType="submit" loading={loading}>
                {editingGroup ? 'Update' : 'Create'}
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default ScheduleGroupConfig;
