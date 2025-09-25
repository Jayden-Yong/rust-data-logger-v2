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
  Popconfirm,
} from 'antd';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  InfoCircleOutlined,
  SettingOutlined,
  SearchOutlined,
  DatabaseOutlined,
  CloudUploadOutlined,
  CloudOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  BarChartOutlined,
  DownloadOutlined,
  FileTextOutlined,
  FolderOpenOutlined,
  ExportOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import axios from 'axios';
import DeviceModelBrowser from './DeviceModelBrowser';

const { Option } = Select;
const { Text } = Typography;

const EnhancedDeviceConfig = () => {
  const [devices, setDevices] = useState([]);
  const [unsyncedDevices, setUnsyncedDevices] = useState([]);
  const [syncedDevices, setSyncedDevices] = useState([]);
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
  
  // Device Groups state
  const [deviceGroups, setDeviceGroups] = useState([]);
  const [selectedDeviceGroup, setSelectedDeviceGroup] = useState(null);
  const [deviceGroupsLoading, setDeviceGroupsLoading] = useState(false);

  // Sync to ThingsBoard state
  const [syncLoading, setSyncLoading] = useState(false);
  const [syncResults, setSyncResults] = useState(null);

  // Generate catalog state
  const [catalogLoading, setCatalogLoading] = useState(false);

  // File management state
  const [catalogFiles, setCatalogFiles] = useState([]);
  const [filesLoading, setFilesLoading] = useState(false);
  const [filesModalVisible, setFilesModalVisible] = useState(false);

  // Fetch device models on component mount
  useEffect(() => {
    fetchDeviceModels();
    fetchScheduleGroups();
    fetchDevices();
    fetchUnsyncedDevices();
    fetchDeviceGroups();
  }, []);

  // Fetch synced devices when group selection changes
  useEffect(() => {
    fetchSyncedDevicesForGroup(selectedDeviceGroup);
  }, [selectedDeviceGroup]);

  const fetchDeviceGroups = async () => {
    try {
      setDeviceGroupsLoading(true);
      console.log('EnhancedDeviceConfig: Fetching device groups...');
      const response = await axios.get('/api/thingsboard/entity-groups?group_type=DEVICE');
      if (response.data.success) {
        const groups = response.data.data;
        console.log(`EnhancedDeviceConfig: Loaded ${groups.length} device groups:`, groups);
        setDeviceGroups(groups);
      } else {
        console.error('EnhancedDeviceConfig: Failed to fetch device groups:', response.data.error);
        message.error('Failed to fetch device groups');
      }
    } catch (error) {
      console.error('EnhancedDeviceConfig: Failed to fetch device groups:', error);
      message.error('Failed to fetch device groups');
    } finally {
      setDeviceGroupsLoading(false);
    }
  };

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
        console.log('EnhancedDeviceConfig: Loaded devices:', response.data.data);
        setDevices(response.data.data);
      }
    } catch (error) {
      message.error('Failed to fetch devices');
    } finally {
      setLoading(false);
    }
  };

  const fetchUnsyncedDevices = async () => {
    try {
      const response = await axios.get('/api/devices-unsynced');
      if (response.data.success) {
        console.log('EnhancedDeviceConfig: Loaded unsynced devices:', response.data.data);
        setUnsyncedDevices(response.data.data);
      }
    } catch (error) {
      message.error('Failed to fetch unsynced devices');
    }
  };

  const fetchSyncedDevicesForGroup = async (groupId) => {
    if (!groupId) {
      setSyncedDevices([]);
      return;
    }
    
    try {
      const response = await axios.get(`/api/devices-by-group/${groupId}`);
      if (response.data.success) {
        console.log('EnhancedDeviceConfig: Loaded synced devices for group:', groupId, response.data.data);
        setSyncedDevices(response.data.data);
      }
    } catch (error) {
      message.error('Failed to fetch synced devices for group');
      setSyncedDevices([]);
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

      console.log('Fetching tag templates for model:', model.name, 'with ID:', modelId);
      const response = await axios.get(`/api/modbus-tcp-tag-registers?model_id=${encodeURIComponent(modelId)}`);
      if (response.data.success) {
        // Transform the data to match the expected format
        const transformedData = response.data.data.map(item => ({
          id: item.id,
          name: item.data_label,
          address: item.address,
          size: item.size,
          data_type: item.modbus_type,
          description: `${item.ava_type}${item.mppt ? ` - MPPT ${item.mppt}` : ''}${item.input ? ` - Input ${item.input}` : ''} (${item.device_model})`,
          scaling_multiplier: 1.0 / item.divider,  // Convert divider to multiplier (1/divider)
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
      const defaultScheduleGroup = scheduleGroups.find(group => group.id === 'low_freq') || scheduleGroups[0];
      const newTags = tagTemplates.map(template => ({
        name: template.name,
        address: template.address,
        size: template.size || 1,
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

  const syncDevicesToThingsBoard = async () => {
    if (!selectedDeviceGroup) {
      message.error('Please select a device group first');
      return;
    }

    try {
      setSyncLoading(true);
      setSyncResults(null);
      
      message.info('Starting device sync to ThingsBoard...');

      // Call the backend API to sync devices
      const response = await axios.post('/api/sync-devices-to-thingsboard', {
        entity_group_id: selectedDeviceGroup
      });

      if (response.data.success) {
        const results = response.data.data;
        setSyncResults(results);
        
        // Enhanced success message with device ID update info
        const updateInfo = results.updated_device_ids && results.updated_device_ids.length > 0 
          ? `, IDs Updated: ${results.updated_device_ids.length}` 
          : '';
        const updateFailures = results.update_failed_count > 0 
          ? `, Update Failures: ${results.update_failed_count}` 
          : '';
        
        // Create device type summary for the success message
        let deviceTypeSummary = '';
        if (results.updated_device_ids && results.updated_device_ids.length > 0) {
          const deviceTypes = results.updated_device_ids.reduce((acc, device) => {
            acc[device.device_type] = (acc[device.device_type] || 0) + 1;
            return acc;
          }, {});
          
          const typeStrs = Object.entries(deviceTypes).map(([type, count]) => `${count} ${type}${count > 1 ? 's' : ''}`);
          if (typeStrs.length > 0) {
            deviceTypeSummary = ` (${typeStrs.join(', ')})`;
          }
        }
        
        message.success(
          `Sync completed! Created: ${results.created_count}${deviceTypeSummary}, Failed: ${results.failed_count}${updateInfo}${updateFailures}`
        );
        
        // Show detailed results in a modal if there were device ID updates
        if (results.updated_device_ids && results.updated_device_ids.length > 0) {
          Modal.success({
            title: 'Device Sync Completed with ID Updates',
            width: 700,
            content: (
              <div style={{ padding: '16px 0' }}>
                <Card size="small" style={{ marginBottom: '16px', backgroundColor: '#f6ffed', border: '1px solid #b7eb8f' }}>
                  <Space direction="vertical" style={{ width: '100%' }}>
                    <Text strong>Sync Summary:</Text>
                    <div>
                      <Space>
                        <Text>Total Devices:</Text>
                        <Tag color="blue">{results.total_devices}</Tag>
                      </Space>
                      <Space>
                        <Text>Created:</Text>
                        <Tag color="green">{results.created_count}</Tag>
                      </Space>
                      <Space>
                        <Text>Failed:</Text>
                        <Tag color="red">{results.failed_count}</Tag>
                      </Space>
                      <Space>
                        <Text>IDs Updated:</Text>
                        <Tag color="purple">{results.updated_device_ids.length}</Tag>
                      </Space>
                    </div>
                  </Space>
                </Card>
                
                {results.updated_device_ids.length > 0 && (
                  <Card size="small" style={{ backgroundColor: '#e6f7ff', border: '1px solid #91d5ff' }}>
                    <Text strong style={{ color: '#1890ff' }}>ThingsBoard ID Updates:</Text>
                    <div style={{ marginTop: '8px', maxHeight: '200px', overflow: 'auto' }}>
                      {results.updated_device_ids.map((update, index) => (
                        <div key={index} style={{ padding: '4px 0', borderBottom: '1px solid #f0f0f0' }}>
                          <Space direction="vertical" size="small">
                            <div>
                              <Text strong>{update.device_name}</Text>
                              <Tag color="blue" style={{ marginLeft: 8 }}>{update.device_type}</Tag>
                            </div>
                            <div style={{ paddingLeft: '16px' }}>
                              <Text type="secondary">Local ID: </Text>
                              <Text code style={{ fontSize: '11px' }}>{update.local_id}</Text>
                              <br />
                              <Text type="secondary">ThingsBoard ID: </Text>
                              <Text code style={{ fontSize: '11px', color: '#52c41a' }}>{update.thingsboard_id}</Text>
                            </div>
                          </Space>
                        </div>
                      ))}
                    </div>
                  </Card>
                )}
                
                {results.update_failed_count > 0 && (
                  <div style={{ marginTop: '12px', padding: '8px', backgroundColor: '#fff2e8', borderRadius: '4px', border: '1px solid #ffd591' }}>
                    <Text style={{ color: '#fa8c16' }}>
                      Warning: {results.update_failed_count} device(s) could not be linked with ThingsBoard IDs in local database. Devices were created in ThingsBoard but local correlation is missing.
                    </Text>
                  </div>
                )}
              </div>
            ),
          });
        }
        
        console.log('Sync results:', results);
        
        // Always refresh the device lists after sync completion to show updated status
        await fetchDevices();
        await fetchUnsyncedDevices();
        await fetchSyncedDevicesForGroup(selectedDeviceGroup);
      } else {
        message.error(`Sync failed: ${response.data.error || 'Unknown error'}`);
        
        // Still refresh even on failure to show current state
        await fetchUnsyncedDevices();
        await fetchSyncedDevicesForGroup(selectedDeviceGroup);
      }
    } catch (error) {
      console.error('Sync error:', error);
      message.error(`Failed to sync devices: ${error.message}`);
      
      // Refresh on error as well to ensure UI shows current state
      await fetchUnsyncedDevices();
      await fetchSyncedDevicesForGroup(selectedDeviceGroup);
    } finally {
      setSyncLoading(false);
    }
  };

  const generateDeviceCatalog = async () => {
    if (!selectedDeviceGroup) {
      message.error('Please select a device group first');
      return;
    }

    try {
      setCatalogLoading(true);
      
      message.info('Generating device catalog with detailed tag information...');

      // Call the backend API to generate device catalog
      const response = await axios.post('/api/generate-device-catalog', {
        entity_group_id: selectedDeviceGroup,
        output_dir: 'catalogs'
      });

      if (response.data.success) {
        const result = response.data.data.message; // Extract the message field from the response
        const selectedGroup = deviceGroups.find(g => g.id.id === selectedDeviceGroup);
        const groupName = selectedGroup?.name || 'Unknown';
        
        // Create safe filename from group name
        const safeGroupName = groupName
          .replace(/\s+/g, '-')
          .replace(/[\/\\:*?"<>|]/g, '-');
        
        const fileName = `${safeGroupName}-device-catalog.csv`;
        const fullPath = `catalogs/${fileName}`;
        
        message.success('Device catalog generated successfully! Check the detailed breakdown in the modal.');
        
        console.log('Catalog generation result:', result);
        
        // Parse parent devices from the result
        const parseParentDevices = (resultText) => {
          const devices = [];
          const lines = resultText.split('\n');
          let inParentSection = false;
          
          for (const line of lines) {
            if (line.includes('🏭 Parent Devices Generated:')) {
              inParentSection = true;
              continue;
            }
            
            if (inParentSection && line.trim() === '') {
              break; // End of parent devices section
            }
            
            if (inParentSection && (line.trim().startsWith('⚡') || line.trim().startsWith('📊') || line.trim().startsWith('📏') || line.trim().startsWith('🌤️'))) {
              // Parse line like "  ⚡ Inverter (1): ACCV-P002-I01" or "  🌤️ Weather Station (1): ACCV-P002-WS01"
              const match = line.match(/\s*([⚡📊📏🌤️🔧])\s+(.+?)\s+\((\d+)\):\s*(.+)/);
              if (match) {
                const [, emoji, deviceType, count, deviceNames] = match;
                const names = deviceNames.split(', ').map(name => name.trim());
                devices.push({
                  type: deviceType,
                  emoji: emoji,
                  count: parseInt(count),
                  devices: names
                });
              }
            }
          }
          return devices;
        };
        
        // Get icon component for device type
        const getDeviceIcon = (deviceType) => {
          switch (deviceType) {
            case 'Inverter':
              return <DatabaseOutlined style={{ color: '#1890ff' }} />;
            case 'PowerMeter':
              return <BarChartOutlined style={{ color: '#52c41a' }} />;
            case 'Meter':
              return <InfoCircleOutlined style={{ color: '#fa8c16' }} />;
            case 'Weather Station':
              return <CloudOutlined style={{ color: '#13c2c2' }} />;
            default:
              return <SettingOutlined style={{ color: '#722ed1' }} />;
          }
        };
        
        const parentDevices = parseParentDevices(result);
        
        // Show success modal with device cards
        Modal.success({
          title: 'Device Catalog Generated Successfully!',
          width: 800,
          content: (
            <div style={{ padding: '16px 0' }}>
              <Card 
                size="small" 
                style={{ 
                  backgroundColor: '#f6ffed', 
                  border: '1px solid #b7eb8f',
                  marginBottom: '16px'
                }}
              >
                <Space direction="vertical" style={{ width: '100%' }} size="middle">
                  <div>
                    <Space>
                      <DatabaseOutlined style={{ color: '#1890ff', fontSize: '16px' }} />
                      <Text strong style={{ color: '#1890ff' }}>Entity Group:</Text>
                      <Tag color="blue">{groupName}</Tag>
                    </Space>
                  </div>
                  
                  <div>
                    <Space>
                      <FileTextOutlined style={{ color: '#fa8c16', fontSize: '16px' }} />
                      <Text strong style={{ color: '#fa8c16' }}>File:</Text>
                      <Text code>{fullPath}</Text>
                    </Space>
                  </div>
                  
                  {parentDevices.length > 0 && (
                    <div>
                      <Text strong style={{ color: '#52c41a', fontSize: '16px', marginBottom: '12px', display: 'block' }}>
                        <CheckCircleOutlined style={{ marginRight: '8px' }} />
                        Devices Included:
                      </Text>
                      <div style={{ 
                        display: 'grid', 
                        gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', 
                        gap: '12px',
                        marginTop: '12px'
                      }}>
                        {parentDevices.map((deviceGroup, index) => (
                          <Card 
                            key={index}
                            size="small" 
                            style={{ 
                              border: '1px solid #d9d9d9',
                              borderRadius: '8px',
                              backgroundColor: '#fafafa'
                            }}
                            title={
                              <Space>
                                {getDeviceIcon(deviceGroup.type)}
                                <Text strong>{deviceGroup.type}</Text>
                                <Tag color="blue">{deviceGroup.count}</Tag>
                              </Space>
                            }
                          >
                            <div style={{ maxHeight: '120px', overflowY: 'auto' }}>
                              {deviceGroup.devices.map((deviceName, deviceIndex) => (
                                <Tag 
                                  key={deviceIndex}
                                  color="green" 
                                  style={{ 
                                    margin: '4px 6px 4px 0',
                                    fontSize: '13px',
                                    padding: '4px 8px',
                                    borderRadius: '6px',
                                    fontWeight: '500'
                                  }}
                                >
                                  {deviceName}
                                </Tag>
                              ))}
                            </div>
                          </Card>
                        ))}
                      </div>
                    </div>
                  )}
                </Space>
              </Card>
            </div>
          ),
        });
      } else {
        message.error(`Failed to generate catalog: ${response.data.error || 'Unknown error'}`);
      }
    } catch (error) {
      console.error('Catalog generation error:', error);
      message.error(`Failed to generate catalog: ${error.message}`);
    } finally {
      setCatalogLoading(false);
    }
  };

  // File Management Functions
  const fetchCatalogFiles = async () => {
    try {
      setFilesLoading(true);
      const response = await axios.get('/api/files/catalogs');
      if (response.data.success) {
        setCatalogFiles(response.data.data);
      } else {
        message.error('Failed to fetch catalog files');
      }
    } catch (error) {
      console.error('Failed to fetch catalog files:', error);
      message.error('Failed to fetch catalog files');
    } finally {
      setFilesLoading(false);
    }
  };

  const downloadFile = async (filename) => {
    try {
      const response = await axios.get(`/api/files/catalogs/${filename}`, {
        responseType: 'blob',
      });
      
      // Create download link
      const url = window.URL.createObjectURL(new Blob([response.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', filename);
      document.body.appendChild(link);
      link.click();
      link.remove();
      window.URL.revokeObjectURL(url);
      
      message.success(`Downloaded ${filename}`);
    } catch (error) {
      console.error('Failed to download file:', error);
      message.error(`Failed to download ${filename}`);
    }
  };

  const deleteFile = async (filename) => {
    try {
      const response = await axios.delete(`/api/files/catalogs/${filename}`);
      
      if (response.data.success) {
        message.success(`File '${filename}' deleted successfully`);
        // Refresh the file list
        await fetchCatalogFiles();
      } else {
        message.error(response.data.error || 'Failed to delete file');
      }
    } catch (error) {
      console.error('Error deleting file:', error);
      message.error('Failed to delete file');
    }
  };

  const confirmDelete = (filename) => {
    Modal.confirm({
      title: 'Delete File',
      content: `Are you sure you want to delete '${filename}'? This action cannot be undone.`,
      okText: 'Delete',
      okType: 'danger',
      cancelText: 'Cancel',
      onOk: () => deleteFile(filename),
    });
  };

  const showFilesModal = async () => {
    setFilesModalVisible(true);
    await fetchCatalogFiles();
  };

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
        // No common_address needed - IEC 104 will read all available IOAs
      }

      // Generate device ID based on protocol configuration
      let deviceId;
      if (editingDevice) {
        // Keep existing ID for updates
        deviceId = editingDevice.device.id;
      } else {
        // Generate new ID for new devices
        if (values.protocol_type === 'modbus_tcp' || values.protocol_type === 'modbus_rtu') {
          // For Modbus protocols, use a combination of protocol type and device ID
          deviceId = `${values.protocol_type}_${values.slave_id}_${Date.now()}`;
        } else if (values.protocol_type === 'iec104') {
          // For IEC 104, use host and timestamp
          const hostPart = values.host.replace(/\./g, '_');
          deviceId = `iec104_${hostPart}_${Date.now()}`;
        } else {
          // Fallback
          deviceId = `device_${Date.now()}`;
        }
      }

      const deviceData = {
        id: deviceId,
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
        response = await axios.put(`/api/devices-enhanced/${deviceId}`, deviceData);
      } else {
        // Create new device
        response = await axios.post('/api/devices-enhanced', deviceData);
      }

      if (response.data.success) {
        message.success(editingDevice ? 'Device updated successfully' : 'Device created successfully');
        setModalVisible(false);
        fetchDevices();
        fetchUnsyncedDevices();
        fetchSyncedDevicesForGroup(selectedDeviceGroup);
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
    const defaultScheduleGroup = scheduleGroups.find(group => group.id === 'low_freq') || scheduleGroups[0];
    const newTag = {
      name: '',
      address: 1,
      size: 1,
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

  const handleDelete = async (deviceId) => {
    try {
      setLoading(true);
      console.log('Attempting to delete device with ID:', deviceId);
      const response = await axios.delete(`/api/devices/${deviceId}`);
      
      if (response.data.success) {
        message.success('Device deleted successfully');
        fetchDevices(); // Refresh the device list
        fetchUnsyncedDevices();
        fetchSyncedDevicesForGroup(selectedDeviceGroup);
      } else {
        console.log('Delete failed with response:', response.data);
        message.error(`Failed to delete device: ${response.data.error || 'Unknown error'}`);
      }
    } catch (error) {
      console.error('Error deleting device:', error);
      console.log('Error response:', error.response?.data);
      const errorMessage = error.response?.data?.error || error.message || 'Unknown error occurred';
      message.error(`Failed to delete device: ${errorMessage}`);
    } finally {
      setLoading(false);
    }
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
        console.log('EnhancedDeviceConfig: Rendering model for ID:', modelId, 'Available models:', deviceModels.length);
        if (!modelId) return <Tag color="default">Custom</Tag>;
        const model = deviceModels.find(m => m.id === modelId);
        console.log('EnhancedDeviceConfig: Found model:', model);
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
        console.log('EnhancedDeviceConfig: Rendering protocol for config:', config);
        try {
          const protocolConfig = JSON.parse(config);
          console.log('EnhancedDeviceConfig: Parsed protocol config:', protocolConfig);
          return <Tag color={getProtocolTypeColor(protocolConfig.type)}>{protocolConfig.type?.toUpperCase()}</Tag>;
        } catch (e) {
          console.error('EnhancedDeviceConfig: Error parsing protocol config:', e);
          return <Tag color="default">Unknown</Tag>;
        }
      },
    },
    {
      title: 'Host IP',
      dataIndex: ['device', 'protocol_config'],
      key: 'host_ip',
      render: (config) => {
        try {
          const protocolConfig = JSON.parse(config);
          if (protocolConfig.host) {
            return <Text code>{protocolConfig.host}</Text>;
          } else if (protocolConfig.type === 'iec104' && protocolConfig.target_host) {
            return <Text code>{protocolConfig.target_host}</Text>;
          }
          return <Text type="secondary">N/A</Text>;
        } catch (e) {
          return <Text type="secondary">N/A</Text>;
        }
      },
    },
    {
      title: 'Device ID',
      dataIndex: ['device', 'protocol_config'],
      key: 'device_id',
      render: (config) => {
        try {
          const protocolConfig = JSON.parse(config);
          if (protocolConfig.type === 'modbus_tcp' || protocolConfig.type === 'modbus_rtu') {
            return <Text code>{protocolConfig.slave_id || 'N/A'}</Text>;
          } else if (protocolConfig.type === 'iec104') {
            return <Text code>{protocolConfig.common_address || 'N/A'}</Text>;
          }
          return <Text type="secondary">N/A</Text>;
        } catch (e) {
          return <Text type="secondary">N/A</Text>;
        }
      },
    },
    {
      title: 'Port',
      dataIndex: ['device', 'protocol_config'],
      key: 'port',
      render: (config) => {
        try {
          const protocolConfig = JSON.parse(config);
          if (protocolConfig.port) {
            return <Text code>{protocolConfig.port}</Text>;
          } else if (protocolConfig.type === 'iec104' && protocolConfig.target_port) {
            return <Text code>{protocolConfig.target_port}</Text>;
          }
          return <Text type="secondary">N/A</Text>;
        } catch (e) {
          return <Text type="secondary">N/A</Text>;
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
          <Popconfirm
            title="Delete Device"
            description="Are you sure you want to delete this device? This action cannot be undone."
            onConfirm={() => handleDelete(record.device.id)}
            okText="Yes"
            cancelText="No"
          >
            <Button
              type="text"
              danger
              icon={<DeleteOutlined />}
            />
          </Popconfirm>
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
      title: (
        <Tooltip title="Number of registers to read for this tag (typically 1 for 16-bit values, 2 for 32-bit values like F32)">
          Size
        </Tooltip>
      ),
      dataIndex: 'size',
      key: 'size',
      width: 80,
      render: (value, record, index) => (
        <InputNumber
          value={value || 1}
          onChange={(val) => updateTag(index, 'size', val || 1)}
          min={1}
          max={4}
          placeholder="1"
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
                <DatabaseOutlined />
                ThingsBoard Device Groups
              </Space>
            }
            extra={
              <Button 
                icon={<SearchOutlined />} 
                onClick={fetchDeviceGroups}
                loading={deviceGroupsLoading}
              >
                Refresh Groups
              </Button>
            }
          >
            <div style={{ marginBottom: 16 }}>
              <Text type="secondary">
                Select a ThingsBoard device group to view associated devices and manage group settings.
              </Text>
            </div>

            <Row gutter={[16, 16]}>
              <Col span={24}>
                <Select
                  placeholder="Select a device group"
                  style={{ width: '100%' }}
                  loading={deviceGroupsLoading}
                  value={selectedDeviceGroup}
                  onChange={setSelectedDeviceGroup}
                  allowClear
                  showSearch={false}  // DISABLE to prevent input errors
                  // REMOVE filterOption to prevent input processing
                  disabled={deviceGroupsLoading}  // ADD to prevent interaction during loading
                  notFoundContent={deviceGroupsLoading ? "Loading..." : "No groups found"}
                >
                  {deviceGroups.map(group => (
                    <Option key={group.id.id} value={group.id.id}>
                      <Space>
                        <DatabaseOutlined style={{ color: '#1890ff', fontSize: '14px' }} />
                        <Text>{group.name}</Text>
                      </Space>
                    </Option>
                  ))}
                </Select>
              </Col>
            </Row>

            {selectedDeviceGroup && (
              <>
                <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
                  <Col span={24}>
                    <Space>
                      <Text strong>Selected Group:</Text>
                      <Tag color="green">
                        {deviceGroups.find(g => g.id.id === selectedDeviceGroup)?.name || 'Unknown'}
                      </Tag>
                    </Space>
                  </Col>
                </Row>
                
                <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
                  <Col>
                    <Space>
                      <Button
                        type="primary"
                        icon={<CloudUploadOutlined />}
                        loading={syncLoading}
                        onClick={syncDevicesToThingsBoard}
                      >
                        {syncLoading ? 'Syncing...' : 'Sync to ThingsBoard'}
                      </Button>
                      
                      <Button
                        type="default"
                        icon={<ExportOutlined />}
                        loading={catalogLoading}
                        onClick={generateDeviceCatalog}
                      >
                        {catalogLoading ? 'Generating Catalog...' : 'Generate Catalog'}
                      </Button>
                      
                      <Button
                        icon={<FileTextOutlined />}
                        onClick={showFilesModal}
                      >
                        Browse Files
                      </Button>
                    </Space>
                  </Col>
                </Row>

                {syncResults && (
                  <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
                    <Col span={24}>
                      <Card size="small" style={{ border: '1px solid #d9d9d9' }}>
                        <Space direction="vertical" style={{ width: '100%' }}>
                          <Text strong>Sync Results:</Text>
                          <Space>
                            <Tag color="green" style={{ fontSize: '14px', padding: '4px 12px', fontWeight: 'bold' }}>
                              <CheckCircleOutlined /> Created: {syncResults.created_count}
                            </Tag>
                            <Tag color="red" style={{ fontSize: '14px', padding: '4px 12px', fontWeight: 'bold' }}>
                              <CloseCircleOutlined /> Failed: {syncResults.failed_count}
                            </Tag>
                            <Tag color="blue" style={{ fontSize: '14px', padding: '4px 12px', fontWeight: 'bold' }}>
                              <BarChartOutlined /> Total: {syncResults.total_devices}
                            </Tag>
                          </Space>
                          {syncResults.failed_devices && syncResults.failed_devices.length > 0 && (
                            <div>
                              <Text strong style={{ color: '#ff4d4f' }}>Failed Devices:</Text>
                              {syncResults.failed_devices.map((failure, index) => (
                                <div key={index} style={{ marginLeft: 16, fontSize: '12px' }}>
                                  • {failure.device_name}: {failure.error}
                                </div>
                              ))}
                            </div>
                          )}
                        </Space>
                      </Card>
                    </Col>
                  </Row>
                )}
              </>
            )}
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col span={24}>
          <Card
            title={
              <Space>
                <CheckCircleOutlined style={{ color: '#52c41a' }} />
                Synced Devices
                {selectedDeviceGroup && (
                  <Tag color="green">
                    {deviceGroups.find(g => g.id.id === selectedDeviceGroup)?.name || 'Group'}
                  </Tag>
                )}
              </Space>
            }
          >
            {!selectedDeviceGroup ? (
              <div style={{ textAlign: 'center', padding: '40px 0', color: '#8c8c8c' }}>
                <CloudUploadOutlined style={{ fontSize: '48px', marginBottom: '16px' }} />
                <div>No group selected</div>
                <div>Select a ThingsBoard device group above to view synced devices</div>
              </div>
            ) : syncedDevices.length === 0 ? (
              <div style={{ textAlign: 'center', padding: '40px 0', color: '#8c8c8c' }}>
                <DatabaseOutlined style={{ fontSize: '48px', marginBottom: '16px' }} />
                <div>No synced devices in this group</div>
                <div>Sync devices to this group using the sync button above</div>
              </div>
            ) : (
              <Table
                columns={deviceColumns}
                dataSource={syncedDevices}
                loading={loading}
                rowKey={(record) => record.device.id}
                pagination={false}
              />
            )}
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col span={24}>
          <Card
            title={
              <Space>
                <SettingOutlined />
                Unsynced Devices
              </Space>
            }
            extra={
              <Button type="primary" icon={<PlusOutlined />} onClick={showAddModal}>
                Add Device
              </Button>
            }
          >
            {unsyncedDevices.length === 0 ? (
              <div style={{ textAlign: 'center', padding: '40px 0', color: '#8c8c8c' }}>
                <DatabaseOutlined style={{ fontSize: '48px', marginBottom: '16px' }} />
                <div>No unsynced devices found</div>
                <div>Create a new device or sync existing devices to ThingsBoard</div>
              </div>
            ) : (
              <Table
                columns={deviceColumns}
                dataSource={unsyncedDevices}
                loading={loading}
                rowKey={(record) => record.device.id}
                pagination={false}
              />
            )}
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
            <Col span={24}>
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
                        label="Device ID"
                        rules={[{ required: true, message: 'Please enter device ID' }]}
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
                        label="Device ID"
                        rules={[{ required: true, message: 'Please enter device ID' }]}
                      >
                        <InputNumber min={1} max={255} placeholder="1" style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                  </Row>
                );
              } else if (protocolType === 'iec104') {
                return (
                  <Row gutter={16}>
                    <Col span={12}>
                      <Form.Item
                        name="host"
                        label="Host"
                        rules={[{ required: true, message: 'Please enter host' }]}
                      >
                        <Input placeholder="192.168.1.100" />
                      </Form.Item>
                    </Col>
                    <Col span={12}>
                      <Form.Item
                        name="port"
                        label="Port"
                        rules={[{ required: true, message: 'Please enter port' }]}
                      >
                        <InputNumber min={1} max={65535} placeholder="2404" style={{ width: '100%' }} />
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

      {/* File Browser Modal */}
      <Modal
        title={
          <Space>
            <FolderOpenOutlined style={{ color: '#1890ff' }} />
            Generated CSV Files
          </Space>
        }
        open={filesModalVisible}
        onCancel={() => setFilesModalVisible(false)}
        width={800}
        footer={[
          <Button key="refresh" icon={<ReloadOutlined />} onClick={fetchCatalogFiles} loading={filesLoading}>
            Refresh
          </Button>,
          <Button key="close" onClick={() => setFilesModalVisible(false)}>
            Close
          </Button>
        ]}
      >
        <div style={{ marginBottom: 16 }}>
          <Text type="secondary">
            All generated CSV catalog files are listed below. Click download to save them to your local computer.
          </Text>
        </div>
        
        {filesLoading ? (
          <div style={{ textAlign: 'center', padding: '40px 0' }}>
            <ReloadOutlined style={{ fontSize: '48px', color: '#1890ff', marginBottom: '16px' }} />
            <div>Loading files...</div>
          </div>
        ) : catalogFiles.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '40px 0', color: '#8c8c8c' }}>
            <FolderOpenOutlined style={{ fontSize: '48px', marginBottom: '16px' }} />
            <div>No CSV files found</div>
            <div>Generate a device catalog first to see files here</div>
          </div>
        ) : (
          <Table
            dataSource={catalogFiles}
            pagination={{ pageSize: 10, showSizeChanger: true }}
            rowKey="name"
            columns={[
              {
                title: 'File Name',
                dataIndex: 'name',
                key: 'name',
                render: (name) => (
                  <Space>
                    <FileTextOutlined style={{ color: '#52c41a' }} />
                    <Text strong>{name}</Text>
                  </Space>
                ),
              },
              {
                title: 'Size',
                dataIndex: 'size',
                key: 'size',
                width: 100,
                render: (size) => {
                  if (size < 1024) return `${size} B`;
                  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
                  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
                },
              },
              {
                title: 'Modified',
                dataIndex: 'modified',
                key: 'modified',
                width: 180,
              },
              {
                title: 'Actions',
                key: 'actions',
                width: 200,
                render: (_, record) => (
                  <Space>
                    <Button
                      type="primary"
                      size="small"
                      icon={<DownloadOutlined />}
                      onClick={() => downloadFile(record.name)}
                    >
                      Download
                    </Button>
                    <Button
                      danger
                      size="small"
                      icon={<DeleteOutlined />}
                      onClick={() => confirmDelete(record.name)}
                    >
                      Delete
                    </Button>
                  </Space>
                ),
              },
            ]}
          />
        )}
      </Modal>
    </div>
  );
};

export default EnhancedDeviceConfig;
