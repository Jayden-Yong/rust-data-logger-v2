import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Form, 
  Input, 
  InputNumber, 
  Select, 
  Button, 
  message,
  Divider,
  Row,
  Col 
} from 'antd';
import { SaveOutlined } from '@ant-design/icons';
import axios from 'axios';

const { Option } = Select;

const SystemConfig = () => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    fetchConfig();
  }, []);

  const fetchConfig = async () => {
    setLoading(true);
    try {
      const response = await axios.get('/api/config');
      if (response.data.success) {
        form.setFieldsValue(response.data.data);
      }
    } catch (error) {
      message.error('Failed to fetch configuration');
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = async (values) => {
    setSaving(true);
    try {
      const response = await axios.post('/api/config', values);
      if (response.data.success) {
        message.success('Configuration saved successfully');
      } else {
        message.error(response.data.error);
      }
    } catch (error) {
      message.error('Failed to save configuration');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <Card
        title="System Configuration"
        extra={
          <Button
            type="primary"
            icon={<SaveOutlined />}
            onClick={() => form.submit()}
            loading={saving}
          >
            Save Configuration
          </Button>
        }
        loading={loading}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
        >
          <Divider>Server Configuration</Divider>
          
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                name={['server', 'host']}
                label="Server Host"
                rules={[{ required: true, message: 'Please enter server host' }]}
              >
                <Input placeholder="0.0.0.0" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                name={['server', 'port']}
                label="Server Port"
                rules={[{ required: true, message: 'Please enter server port' }]}
              >
                <InputNumber min={1} max={65535} placeholder="8080" />
              </Form.Item>
            </Col>
          </Row>

          <Divider>Database Configuration</Divider>
          
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                name={['database', 'path']}
                label="Database Path"
                rules={[{ required: true, message: 'Please enter database path' }]}
              >
                <Input placeholder="data.db" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                name={['database', 'max_log_entries']}
                label="Max Log Entries"
                rules={[{ required: true, message: 'Please enter max log entries' }]}
              >
                <InputNumber min={1000} placeholder="1000000" />
              </Form.Item>
            </Col>
          </Row>

          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                name={['database', 'cleanup_interval_hours']}
                label="Cleanup Interval (hours)"
                rules={[{ required: true, message: 'Please enter cleanup interval' }]}
              >
                <InputNumber min={1} placeholder="24" />
              </Form.Item>
            </Col>
          </Row>

          <Divider>Logging Configuration</Divider>
          
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                name={['logging', 'level']}
                label="Log Level"
                rules={[{ required: true, message: 'Please select log level' }]}
              >
                <Select placeholder="Select log level">
                  <Option value="error">Error</Option>
                  <Option value="warn">Warning</Option>
                  <Option value="info">Info</Option>
                  <Option value="debug">Debug</Option>
                  <Option value="trace">Trace</Option>
                </Select>
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                name={['logging', 'file_path']}
                label="Log File Path"
              >
                <Input placeholder="app.log" />
              </Form.Item>
            </Col>
          </Row>

          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                name={['logging', 'max_file_size_mb']}
                label="Max File Size (MB)"
                rules={[{ required: true, message: 'Please enter max file size' }]}
              >
                <InputNumber min={1} placeholder="10" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                name={['logging', 'max_files']}
                label="Max Files"
                rules={[{ required: true, message: 'Please enter max files' }]}
              >
                <InputNumber min={1} placeholder="5" />
              </Form.Item>
            </Col>
          </Row>
        </Form>
      </Card>
    </div>
  );
};

export default SystemConfig;
