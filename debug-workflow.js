#!/usr/bin/env node

/**
 * Debug script to test the exact workflow and see what data is being passed
 */

const axios = require('axios');

const BASE_URL = 'http://localhost:8080';

async function debugWorkflow() {
  console.log('🔍 Debugging the tag template workflow...\n');

  try {
    // 1. Get device models
    console.log('1. Fetching device models...');
    const modelsResponse = await axios.get(`${BASE_URL}/api/device-models`);
    console.log(`   Found ${modelsResponse.data.data.length} models`);
    
    // Check for duplicate IDs
    const modelIds = modelsResponse.data.data.map(m => m.id);
    const duplicateIds = modelIds.filter((id, index) => modelIds.indexOf(id) !== index);
    if (duplicateIds.length > 0) {
      console.log(`   ⚠️  Duplicate model IDs found: ${duplicateIds.join(', ')}`);
    }
    
    // 2. Find target model
    const targetModel = modelsResponse.data.data.find(m => m.name === 'Sungrow fghj');
    if (!targetModel) {
      console.log('   ❌ Target model not found');
      return;
    }
    
    console.log(`   ✅ Target model: ${targetModel.name} (ID: ${targetModel.id})`);
    
    // 3. Simulate DeviceModelBrowser fetchTagTemplates
    console.log('\n2. Simulating DeviceModelBrowser.fetchTagTemplates...');
    
    const tagResponse = await axios.get(`${BASE_URL}/api/modbus-tcp-tag-registers?device_model=${encodeURIComponent(targetModel.name)}`);
    console.log(`   API Response: ${tagResponse.data.success ? 'success' : 'failed'}`);
    console.log(`   Found ${tagResponse.data.data.length} raw tag registers`);
    
    // 4. Simulate transformation
    console.log('\n3. Simulating data transformation...');
    const transformedData = tagResponse.data.data.map(item => ({
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
    
    console.log(`   ✅ Transformed ${transformedData.length} tag templates`);
    
    // 5. Simulate handleSelectAndClose
    console.log('\n4. Simulating handleSelectAndClose...');
    const modelToPass = {
      ...targetModel,
      label: `${targetModel.manufacturer || 'Various'} - ${targetModel.name}`,
      value: targetModel.id,
      tagTemplates: transformedData,
      tags: transformedData
    };
    
    console.log(`   ✅ Model object created with:`);
    console.log(`      - ID: ${modelToPass.id}`);
    console.log(`      - Name: ${modelToPass.name}`);
    console.log(`      - Label: ${modelToPass.label}`);
    console.log(`      - TagTemplates: ${modelToPass.tagTemplates?.length || 0} items`);
    console.log(`      - Tags: ${modelToPass.tags?.length || 0} items`);
    
    // 6. Simulate EnhancedDeviceConfig onSelectModel
    console.log('\n5. Simulating EnhancedDeviceConfig.onSelectModel...');
    console.log(`   Model received:`, JSON.stringify({
      id: modelToPass.id,
      name: modelToPass.name,
      tagTemplatesLength: modelToPass.tagTemplates?.length,
      tagsLength: modelToPass.tags?.length
    }, null, 2));
    
    // Check what would happen in the if/else logic
    if (modelToPass.tagTemplates && modelToPass.tagTemplates.length > 0) {
      console.log(`   ✅ Would use ${modelToPass.tagTemplates.length} tag templates from model browser`);
      console.log(`   🎯 setTagTemplates would be called with ${modelToPass.tagTemplates.length} items`);
    } else if (modelToPass.tags && modelToPass.tags.length > 0) {
      console.log(`   ✅ Would use ${modelToPass.tags.length} tags from model browser`);
      console.log(`   🎯 setTagTemplates would be called with ${modelToPass.tags.length} items`);
    } else {
      console.log(`   ❌ No tag templates found, would fetch from API`);
    }
    
    // 7. Show what tags would be created
    console.log('\n6. Simulating tag creation for device...');
    const sampleTags = transformedData.slice(0, 3).map(template => ({
      name: template.name,
      address: template.address,
      data_type: template.data_type,
      description: template.description,
      scaling_multiplier: template.scaling_multiplier,
      scaling_offset: template.scaling_offset,
      unit: template.unit,
      read_only: template.read_only,
      enabled: true,
      schedule_group_id: 'medium_freq', // default
    }));
    
    console.log(`   Sample device tags that would be created:`);
    sampleTags.forEach((tag, index) => {
      console.log(`   ${index + 1}. ${tag.name} (${tag.address}) - ${tag.data_type} [${tag.unit}]`);
    });
    
    console.log('\n✅ Debug complete - if this data looks correct, the issue might be in the React rendering or state updates');
    
  } catch (error) {
    console.error('❌ Debug failed:', error.message);
    if (error.response) {
      console.error('   API Response:', error.response.status, error.response.statusText);
    }
  }
}

debugWorkflow();
