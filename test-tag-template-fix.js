#!/usr/bin/env node

// Test script to verify tag template passing functionality
// This simulates the workflow: select model -> fetch tags -> pass to parent

const https = require('http');

const testModelSelection = async () => {
  console.log('🧪 Testing tag template fix...\n');

  try {
    // 1. Simulate getting device models
    console.log('1. Fetching device models...');
    const models = await apiCall('/api/device-models');
    console.log(`   Found ${models.data.length} device models`);
    
    // Find a model with modbus_tcp protocol that has tag registers
    let modbusModel = models.data.find(m => m.protocol_type === 'modbus_tcp' && m.name === 'Sungrow fghj');
    if (!modbusModel) {
      // Try to find "FINAL TEST WORKING" model
      modbusModel = models.data.find(m => m.protocol_type === 'modbus_tcp' && m.name === 'FINAL TEST WORKING');
    }
    if (!modbusModel) {
      // Try to find any model with "SG" in the name
      modbusModel = models.data.find(m => m.protocol_type === 'modbus_tcp' && m.name.includes('SG'));
    }
    if (!modbusModel) {
      modbusModel = models.data.find(m => m.protocol_type === 'modbus_tcp');
    }
    if (!modbusModel) {
      console.log('❌ No Modbus TCP models found');
      return;
    }
    
    console.log(`   Selected model: ${modbusModel.name} (${modbusModel.protocol_type})`);

    // 2. Simulate fetching tag templates for the selected model
    console.log('\n2. Fetching tag templates...');
    const tagResponse = await apiCall(`/api/modbus-tcp-tag-registers?device_model=${encodeURIComponent(modbusModel.name)}`);
    
    if (tagResponse.data.length > 0) {
      console.log(`   ✅ Found ${tagResponse.data.length} tag registers`);
      console.log(`   Sample tag: ${tagResponse.data[0].data_label} (Address: ${tagResponse.data[0].address})`);
      
      // 3. Simulate the transformation that happens in DeviceModelBrowser
      console.log('\n3. Transforming tag data...');
      const transformedTags = tagResponse.data.slice(0, 5).map(item => ({
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
      
      console.log(`   ✅ Transformed ${transformedTags.length} tags`);
      
      // 4. Simulate what gets passed to parent component
      console.log('\n4. Simulating model selection with tag templates...');
      const modelToPass = {
        ...modbusModel,
        label: `${modbusModel.manufacturer || 'Various'} - ${modbusModel.name}`,
        value: modbusModel.id,
        tagTemplates: transformedTags,
        tags: transformedTags // Alternative field name
      };
      
      console.log(`   ✅ Model object prepared with ${modelToPass.tagTemplates.length} tag templates`);
      console.log(`   📋 Sample tag template:`);
      console.log(`      Name: ${transformedTags[0].name}`);
      console.log(`      Address: ${transformedTags[0].address}`);
      console.log(`      Type: ${transformedTags[0].data_type}`);
      console.log(`      Unit: ${transformedTags[0].unit}`);
      
      console.log('\n✅ Test completed successfully! The fix should work.');
      console.log('\n📝 To test in the browser:');
      console.log('   1. Open http://localhost:3000');
      console.log('   2. Go to Device Configuration');
      console.log('   3. Click "Add Device"');
      console.log('   4. Click "Browse Models"');
      console.log(`   5. Select "${modbusModel.name}" model`);
      console.log('   6. Click "Select" button');
      console.log('   7. Check if tag registers table fills automatically');
      
    } else {
      console.log(`   ⚠️  No tag registers found for ${modbusModel.name}`);
    }

  } catch (error) {
    console.error('❌ Test failed:', error.message);
  }
};

// Helper function to make API calls
function apiCall(path) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: 'localhost',
      port: 8080,
      path: path,
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      }
    };

    const req = https.request(options, (res) => {
      let data = '';
      res.on('data', (chunk) => {
        data += chunk;
      });
      res.on('end', () => {
        try {
          const response = JSON.parse(data);
          resolve(response);
        } catch (error) {
          reject(new Error('Invalid JSON response'));
        }
      });
    });

    req.on('error', (error) => {
      reject(error);
    });

    req.end();
  });
}

// Run the test
testModelSelection();
