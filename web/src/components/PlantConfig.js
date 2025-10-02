import React, { useState, useEffect } from 'react';
import { Card, Form, Input, Button, Alert, Space, Typography, Divider, Select, Tag } from 'antd';
import { SettingOutlined, SaveOutlined, DatabaseOutlined, SearchOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useAuth } from '../contexts/AuthContext';

const { Title, Text } = Typography;
const { Option } = Select;

const PlantConfig = () => {
  const { isAdmin } = useAuth();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [config, setConfig] = useState(null);
  const [message, setMessage] = useState(null);
  
  // Device groups state
  const [deviceGroups, setDeviceGroups] = useState([]);
  const [deviceGroupsLoading, setDeviceGroupsLoading] = useState(false);
  const [selectedDeviceGroup, setSelectedDeviceGroup] = useState(null);

  useEffect(() => {
    loadPlantConfig();
    fetchDeviceGroups();
  }, []);

  const fetchDeviceGroups = async () => {
    try {
      setDeviceGroupsLoading(true);
      console.log('PlantConfig: Fetching device groups...');
      const response = await axios.get('/api/thingsboard/entity-groups?group_type=DEVICE');
      if (response.data.success) {
        const groups = response.data.data;
        console.log(`PlantConfig: Loaded ${groups.length} device groups:`, groups);
        setDeviceGroups(groups);
      } else {
        console.error('PlantConfig: Failed to fetch device groups:', response.data.error);
        setMessage({ type: 'warning', text: 'Failed to fetch device groups from ThingsBoard' });
      }
    } catch (error) {
      console.error('PlantConfig: Failed to fetch device groups:', error);
      setMessage({ type: 'warning', text: 'Failed to connect to ThingsBoard for device groups' });
    } finally {
      setDeviceGroupsLoading(false);
    }
  };

  const loadPlantConfig = async () => {
    setLoading(true);
    try {
      const response = await axios.get('/api/plant-config');
      if (response.data.success) {
        const plantConfig = response.data.data;
        setConfig(plantConfig);
        
        // Set the selected device group for the selector
        if (plantConfig.thingsboard_entity_group_id) {
          setSelectedDeviceGroup(plantConfig.thingsboard_entity_group_id);
        }
        
        form.setFieldsValue({
          plant_name: plantConfig.plant_name,
          thingsboard_entity_group_id: plantConfig.thingsboard_entity_group_id || '',
        });
      } else {
        setMessage({ type: 'error', text: response.data.error });
      }
    } catch (error) {
      console.error('Failed to load plant configuration:', error);
      setMessage({ type: 'error', text: 'Failed to load plant configuration' });
    } finally {
      setLoading(false);
    }
  };

  const handleDeviceGroupChange = (groupId) => {
    setSelectedDeviceGroup(groupId);
    
    // Find the selected group and get its details
    const selectedGroup = deviceGroups.find(g => g.id.id === groupId);
    if (selectedGroup) {
      // Update the form field with the group ID
      form.setFieldValue('thingsboard_entity_group_id', groupId);
    } else {
      // Clear the form field if no group selected
      form.setFieldValue('thingsboard_entity_group_id', '');
    }
  };

  const handleSave = async (values) => {
    setSaving(true);
    setMessage(null);

    try {
      // Get the selected device group details
      const selectedGroup = deviceGroups.find(g => g.id.id === selectedDeviceGroup);
      const plantName = selectedGroup ? selectedGroup.name : 'Default Plant';

      const payload = {
        plant_name: plantName, // Use device group name as plant name
        thingsboard_entity_group_id: selectedDeviceGroup || null,
      };

      const response = await axios.post('/api/plant-config', payload);
      
      if (response.data.success) {
        setMessage({ type: 'success', text: 'Plant configuration updated successfully!' });
        setConfig({ ...config, ...payload });
      } else {
        setMessage({ type: 'error', text: response.data.error || 'Failed to update configuration' });
      }
    } catch (error) {
      console.error('Failed to save plant configuration:', error);
      setMessage({ type: 'error', text: 'Failed to save plant configuration' });
    } finally {
      setSaving(false);
    }
  };

  if (!isAdmin) {
    return (
      <Card>
        <Alert
          message="Access Restricted"
          description="You need administrator privileges to view plant configuration."
          type="warning"
          showIcon
        />
      </Card>
    );
  }

  return (
    <div style={{ padding: '24px', maxWidth: '100%', margin: '0 auto' }}>
      <Card
        title={
          <Space>
            <SettingOutlined style={{ color: '#1890ff' }} />
            <span style={{ fontSize: '20px', fontWeight: 'bold' }}>Plant Configuration</span>
          </Space>
        }
        loading={loading}
        style={{
          width: '100%',
          boxShadow: '0 4px 12px rgba(0, 0, 0, 0.1)',
          borderRadius: '12px',
          border: '1px solid #e8e8e8'
        }}
        headStyle={{
          background: 'linear-gradient(135deg, #fafafa 0%, #f5f5f5 100%)',
          borderBottom: '1px solid #e8e8e8',
          borderRadius: '12px 12px 0 0'
        }}
        bodyStyle={{ padding: '32px' }}
      >
        {message && (
          <Alert
            message={message.text}
            type={message.type}
            showIcon
            closable
            onClose={() => setMessage(null)}
            style={{ 
              marginBottom: 24,
              borderRadius: '8px',
              fontSize: '14px'
            }}
          />
        )}

        <div style={{ marginBottom: 24 }}>
          <Title level={4} style={{ color: '#262626', marginBottom: 8 }}>
            ThingsBoard Integration
          </Title>
          <Text type="secondary" style={{ fontSize: '15px', lineHeight: '1.6' }}>
            Select a ThingsBoard device group to associate with this plant. This will filter devices for installer users.
          </Text>
        </div>

        <Form
          form={form}
          layout="vertical"
          onFinish={handleSave}
          disabled={saving}
        >
          <Form.Item
            label={
              <Space style={{ marginBottom: 8 }}>
                <DatabaseOutlined style={{ color: '#1890ff' }} />
                <Text strong style={{ fontSize: '16px' }}>Device Group Selection</Text>
                <Button 
                  type="link" 
                  size="small"
                  icon={<SearchOutlined />} 
                  onClick={fetchDeviceGroups}
                  loading={deviceGroupsLoading}
                  style={{ padding: '0 8px' }}
                >
                  Refresh Groups
                </Button>
              </Space>
            }
            name="thingsboard_entity_group_id"

            rules={[
              { required: true, message: 'Please select a device group' }
            ]}
          >
            <Select
              placeholder="Choose a ThingsBoard device group..."
              style={{ width: '100%', height: 48 }}
              loading={deviceGroupsLoading}
              value={selectedDeviceGroup}
              onChange={handleDeviceGroupChange}
              allowClear
              showSearch={false}
              disabled={deviceGroupsLoading}
              notFoundContent={deviceGroupsLoading ? "Loading device groups..." : "No groups found"}
            >
              {deviceGroups.map(group => (
                <Option key={group.id.id} value={group.id.id}>
                  <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                    <Space>
                      <DatabaseOutlined style={{ color: '#1890ff', fontSize: '16px' }} />
                      <Text strong>{group.name}</Text>
                    </Space>
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      ID: {group.id.id.substring(0, 8)}...
                    </Text>
                  </Space>
                </Option>
              ))}
            </Select>
          </Form.Item>

          <div style={{ marginTop: 32, display: 'flex', justifyContent: 'flex-end' }}>
            <Space size="middle">
              <Button 
                onClick={() => {
                  form.resetFields();
                  setSelectedDeviceGroup(null);
                }}
                disabled={saving}
                style={{ borderRadius: '6px', height: '40px', paddingLeft: '20px', paddingRight: '20px' }}
              >
                Reset
              </Button>
              <Button 
                type="primary" 
                htmlType="submit" 
                loading={saving}
                icon={<SaveOutlined />}
                disabled={!selectedDeviceGroup}
                style={{ 
                  borderRadius: '6px', 
                  height: '40px', 
                  paddingLeft: '24px', 
                  paddingRight: '24px',
                  fontWeight: 'bold'
                }}
              >
                {saving ? 'Saving Configuration...' : 'Save Configuration'}
              </Button>
            </Space>
          </div>
        </Form>

        {config && (
          <>
            <Divider style={{ margin: '32px 0' }} />
            <div style={{ 
              background: '#fafafa', 
              padding: '20px', 
              borderRadius: '8px', 
              border: '1px solid #e8e8e8' 
            }}>
              <Title level={4} style={{ color: '#262626', marginBottom: 16 }}>
                Current Configuration
              </Title>
              <Space direction="vertical" style={{ width: '100%' }} size="middle">
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <Space>
                    <Text strong style={{ color: '#595959' }}>Plant Name:</Text>
                    <Text style={{ fontSize: '15px' }}>{config.plant_name}</Text>
                  </Space>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <Space>
                    <Text strong style={{ color: '#595959' }}>ThingsBoard Device Group:</Text>
                    {config.thingsboard_entity_group_id ? (
                      <Space>
                        <Tag color="blue" style={{ fontSize: '13px', padding: '4px 12px', borderRadius: '6px' }}>
                          <DatabaseOutlined style={{ marginRight: 4 }} />
                          {deviceGroups.find(g => g.id.id === config.thingsboard_entity_group_id)?.name || 'Unknown Group'}
                        </Tag>
                      </Space>
                    ) : (
                      <Text type="secondary">Not configured</Text>
                    )}
                  </Space>
                </div>
                {config.thingsboard_entity_group_id && (
                  <div style={{ marginTop: 12, padding: '12px', background: '#f6ffed', borderRadius: '6px', border: '1px solid #b7eb8f' }}>
                    <Space>
                      <Text type="secondary" style={{ fontSize: '12px' }}>Group ID:</Text>
                      <Text code style={{ fontSize: '12px' }}>{config.thingsboard_entity_group_id}</Text>
                    </Space>
                  </div>
                )}
              </Space>
            </div>
          </>
        )}
      </Card>
    </div>
  );
};

export default PlantConfig;