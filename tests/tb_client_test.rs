use ava_device_logger::tb_rust_client::{ThingsBoardClient, TbError, CreateDeviceRequest, DeviceData, to_thingsboard_device}; // Import the CreateDeviceRequest and new function
use ava_device_logger::database::Database; // Import Database
use std::error::Error;
use chrono::{DateTime, Utc};
use csv::Writer;
use std::fs::File;


fn export_devices_to_csv(devices: &[DeviceData], filename: &str) -> Result<(), Box<dyn Error>> {
    let file = File::create(format!("example_catalogs/{}", filename))?;
    let mut writer = Writer::from_writer(file);

    // Write CSV header
    writer.write_record(&[
        "Device ID",
        "Name", 
        "Type",
        "Label",
        "Tenant ID",
        "Customer ID", 
        "Device Profile ID",
        "Created Time",
        "Entity Type",
        "Additional Info"
    ])?;

    // Write device data
    for device in devices {
        let created_time_str = if let Some(created_time) = device.created_time {
            DateTime::from_timestamp_millis(created_time)
                .unwrap_or_else(|| Utc::now())
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string()
        } else {
            "N/A".to_string()
        };

        let tenant_id = device.tenant_id.as_ref().map(|t| t.id.as_str()).unwrap_or("N/A");
        let customer_id = device.customer_id.as_ref().map(|c| c.id.as_str()).unwrap_or("N/A");
        let profile_id = device.device_profile_id.as_ref().map(|p| p.id.as_str()).unwrap_or("N/A");
        
        let additional_info = if let Some(info) = &device.additional_info {
            if info.is_empty() {
                "None".to_string()
            } else {
                info.iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<_>>()
                    .join("; ")
            }
        } else {
            "None".to_string()
        };

        writer.write_record(&[
            &device.id.id,
            &device.name,
            &device.device_type,
            &device.label,
            tenant_id,
            customer_id,
            profile_id,
            &created_time_str,
            &device.id.entity_type,
            &additional_info,
        ])?;
    }

    writer.flush()?;
    println!("✅ Exported {} devices to {}", devices.len(), filename);
    Ok(())
}


// Login Test
#[tokio::test]
async fn test_login() -> Result<(), TbError> {
    let base_url = "https://monitoring.avaasia.co".to_string();
    let mut client = ThingsBoardClient::new(&base_url);

    client.login("jaydenyong28@gmail.com", "lalala88").await?;
    
    // Check if the token is now set in the client
    match client.get_token() {
        Some(token) => {
            println!("Login successful! Token: {}", token);
            Ok(()) // Return success
        }
        None => {
            Err(TbError::Auth("Login failed: Token not received".to_string()))
        }
    }
}


// Add Device to Device Group Test
#[tokio::test]
async fn test_add_device() -> Result<(), Box<dyn Error>> {
    let entity_group_id = "e695a840-53e1-11f0-8c80-571814f5abd0".to_string();
    let base_url = "https://monitoring.avaasia.co".to_string();
    let device_token = Some("your_device_token".to_string()); // Replace with an actual device token, or set to None

    let mut client = ThingsBoardClient::new(&base_url);
    client.login("jaydenyong28@gmail.com", "lalala88").await?;

    // Define the device properties
    let new_device = CreateDeviceRequest {
        id: None,
        tenant_id: None,
        customer_id: None,
        owner_id: None,
        name: "Test Device".to_string(),
        device_type: "default".to_string(),
        label: Some("Test Device".to_string()),
        device_profile_id: None,
        device_data: None,
        firmware_id: None,
        software_id: None,
        additional_info: None,
    };

    let created_device = client
        .create_device(&new_device, &entity_group_id, device_token.as_deref()) // Pass the entity group ID and device token
        .await?;

    println!("Created device: {:?}", created_device);

    Ok(())
}


// get all devices from entity group test
#[tokio::test]
async fn test_get_devices_by_entity_group() -> Result<(), Box<dyn Error>> {
    let entity_group_id = "f5d00ce0-ff1a-11ef-8171-f30c95d6533b".to_string();
    let base_url = "https://monitoring.avaasia.co".to_string();

    let mut client = ThingsBoardClient::new(&base_url);
    client.login("jaydenyong28@gmail.com", "lalala88").await?;

    let all_devices = client.get_all_group_devices(&entity_group_id, 10).await?;
    
    println!("{}", "=".repeat(80));
    println!("THINGSBOARD ENTITY GROUP DEVICES REPORT");
    println!("{}", "=".repeat(80));
    println!("Entity Group ID: {}", entity_group_id);
    println!("Total devices retrieved: {}", all_devices.len());
    println!("{}", "=".repeat(80));


    // Export to CSV
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let csv_filename = format!("thingsboard_devices_{}.csv", timestamp);
    
    match export_devices_to_csv(&all_devices, &csv_filename) {
        Ok(()) => println!("📄 CSV export successful: {}", csv_filename),
        Err(e) => println!("❌ CSV export failed: {}", e),
    }
    

    if all_devices.is_empty() {
        println!("⚠️  No devices found in this entity group");
    } else {
        // for (index, device) in all_devices.iter().enumerate() {
        //     println!("\n📱 DEVICE #{} ({})", index + 1, device.id.id);
        //     println!("┌─ Name: {}", device.name);
        //     println!("├─ Type: {}", device.device_type);
        //     println!("├─ Label: {}", device.label);
            
        //     if let Some(tenant_id) = &device.tenant_id {
        //         println!("├─ Tenant ID: {}", tenant_id.id);
        //     }
            
        //     if let Some(customer_id) = &device.customer_id {
        //         println!("├─ Customer ID: {}", customer_id.id);
        //     }
            
        //     if let Some(profile_id) = &device.device_profile_id {
        //         println!("├─ Device Profile ID: {}", profile_id.id);
        //     }
            
        //     if let Some(created_time) = device.created_time {
        //         let datetime = DateTime::from_timestamp_millis(created_time)
        //             .unwrap_or_else(|| Utc::now());
        //         println!("├─ Created: {}", datetime.format("%Y-%m-%d %H:%M:%S UTC"));
        //     }
            
        //     if let Some(additional_info) = &device.additional_info {
        //         if !additional_info.is_empty() {
        //             println!("├─ Additional Info:");
        //             for (key, value) in additional_info {
        //                 println!("│  ├─ {}: {}", key, value);
        //             }
        //         }
        //     }
            
        //     println!("└─ Entity Type: {}", device.id.entity_type);
            
        //     if index < all_devices.len() - 1 {
        //         println!("{}", "─".repeat(50));
        //     }
        // }
        println!("\n{}", "=".repeat(80));
        println!("END OF REPORT - {} devices total", all_devices.len());
        println!("{}", "=".repeat(80));
    }

    Ok(())
}


// get all entity groups test
#[tokio::test]
async fn test_get_all_entity_groups() -> Result<(), Box<dyn Error>> {
    let base_url = "https://monitoring.avaasia.co".to_string();

    let mut client = ThingsBoardClient::new(&base_url);
    client.login("jaydenyong28@gmail.com", "lalala88").await?;

    // Test only with DEVICE type
    let group_type = "DEVICE";
    
    println!("\n{}", "=".repeat(80));
    println!("THINGSBOARD ENTITY GROUPS REPORT - TYPE: {}", group_type);
    println!("{}", "=".repeat(80));
    
    let groups = client.get_all_entity_groups(group_type).await?;
    println!("Total entity groups of type '{}': {}", group_type, groups.len());
    
    if groups.is_empty() {
        println!("⚠️  No entity groups found of type '{}'", group_type);
    } else {
        for (index, group) in groups.iter().enumerate() {
            println!("\n🏷️  GROUP #{} ({})", index + 1, group.id.id);
            println!("┌─ Name: {}", group.name);
            println!("├─ Type: {}", group.group_type);
            println!("├─ Owner ID: {}", group.owner_id.id);
            println!("├─ Group All: {}", group.group_all);
            println!("├─ Edge Group All: {}", group.edge_group_all);
            
            if let Some(created_time) = group.created_time {
                let datetime = chrono::DateTime::from_timestamp_millis(created_time)
                    .unwrap_or_else(|| chrono::Utc::now());
                println!("├─ Created: {}", datetime.format("%Y-%m-%d %H:%M:%S UTC"));
            }
            
            if let Some(additional_info) = &group.additional_info {
                if !additional_info.is_empty() {
                    println!("├─ Additional Info:");
                    for (key, value) in additional_info {
                        println!("│  ├─ {}: {}", key, value);
                    }
                }
            }

            if let Some(owner_ids) = &group.owner_ids {
                if !owner_ids.is_empty() {
                    println!("├─ Owner IDs:");
                    for owner in owner_ids {
                        println!("│  ├─ {} ({})", owner.id, owner.entity_type);
                    }
                }
            }
            
            println!("└─ Entity Type: {}", group.id.entity_type);
            
            if index < groups.len() - 1 {
                println!("{}", "─".repeat(50));
            }
        }
    }
    println!("{}", "=".repeat(80));

    Ok(())
}


// Combined test: Get devices from local database and create them in ThingsBoard
#[tokio::test]
async fn test_sync_local_devices_to_thingsboard() -> Result<(), Box<dyn Error>> {
    println!("\n{}", "=".repeat(80));
    println!("SYNC LOCAL DEVICES TO THINGSBOARD TEST");
    println!("{}", "=".repeat(80));

    // Step 1: Connect to local database
    println!("📁 Connecting to local database...");
    let db = Database::new("data.db").await?;
    
    // Step 2: Get all devices from local database
    println!("🔍 Retrieving devices from local database...");
    let local_devices = db.get_devices().await?;
    println!("✅ Found {} devices in local database", local_devices.len());

    if local_devices.is_empty() {
        println!("⚠️  No devices found in local database. Test completed.");
        return Ok(());
    }

    // Step 3: Display local devices
    println!("\n📋 LOCAL DEVICES SUMMARY:");
    for (index, device) in local_devices.iter().enumerate() {
        println!("  {}. {} (ID: {}, Enabled: {}, Type: {})", 
                 index + 1, 
                 device.name, 
                 device.id, 
                 device.enabled,
                 device.model_id.as_deref().unwrap_or("Custom"));
    }

    // Step 4: Connect to ThingsBoard
    println!("\n🌐 Connecting to ThingsBoard...");
    let base_url = "https://monitoring.avaasia.co".to_string();
    let mut tb_client = ThingsBoardClient::new(&base_url);
    tb_client.login("jaydenyong28@gmail.com", "lalala88").await?;
    println!("✅ Successfully authenticated with ThingsBoard");

    // Step 5: Choose an entity group for the devices
    // Using a test entity group - you should replace this with a valid group ID
    let entity_group_id = "e695a840-53e1-11f0-8c80-571814f5abd0"; // ACCV-P002-King Jade group
    println!("🏷️  Target Entity Group: {}", entity_group_id);

    // Step 6: Convert and create devices in ThingsBoard
    println!("\n🚀 Creating devices in ThingsBoard...");
    let mut created_count = 0;
    let mut failed_count = 0;
    let mut results = Vec::new();
    let mut device_type_counters: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for (index, local_device) in local_devices.iter().enumerate() {
        println!("\n📱 Processing device {} of {}: {}", 
                 index + 1, local_devices.len(), local_device.name);

        // Determine device type for counting (similar to API logic)
        let device_type = match serde_json::from_str::<serde_json::Value>(&local_device.protocol_config) {
            Ok(config) => {
                if let Some(protocol_type) = config.get("type").and_then(|v| v.as_str()) {
                    match protocol_type {
                        "modbus_tcp" => "Inverter".to_string(),
                        "modbus_rtu" => "Inverter".to_string(),
                        "iec104" => "Inverter".to_string(),
                        _ => "Inverter".to_string(),
                    }
                } else {
                    "Inverter".to_string()
                }
            }
            Err(_) => "Inverter".to_string(),
        };
        
        // Increment counter for this device type
        let device_index = device_type_counters.entry(device_type.clone()).or_insert(0);
        *device_index += 1;

        // Convert local device to ThingsBoard create request with naming scheme
        let create_request = to_thingsboard_device(local_device, "ACCV-P002-King Jade", *device_index);
        
        // Attempt to create device in ThingsBoard
        match tb_client.create_device(&create_request, entity_group_id, None).await {
            Ok(created_device) => {
                created_count += 1;
                println!("  ✅ Successfully created: {} (TB ID: {})", 
                         created_device.name, 
                         created_device.id.as_ref().map(|id| &id.id).unwrap_or(&String::from("Unknown")));
                results.push((local_device.clone(), Some(created_device), None));
            }
            Err(e) => {
                failed_count += 1;
                println!("  ❌ Failed to create: {} - Error: {}", create_request.name, e);
                results.push((local_device.clone(), None, Some(e.to_string())));
            }
        }

        // Add a small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Step 7: Generate summary report
    println!("\n{}", "=".repeat(80));
    println!("SYNC SUMMARY REPORT");
    println!("{}", "=".repeat(80));
    println!("📊 Total local devices processed: {}", local_devices.len());
    println!("✅ Successfully created in ThingsBoard: {}", created_count);
    println!("❌ Failed to create: {}", failed_count);
    
    if failed_count > 0 {
        println!("\n🚨 FAILED DEVICES:");
        for (device, _, error) in &results {
            if let Some(err) = error {
                println!("  • {} ({}): {}", device.name, device.id, err);
            }
        }
    }

    if created_count > 0 {
        println!("\n🎉 SUCCESSFULLY CREATED DEVICES:");
        for (local_device, tb_device, _) in &results {
            if let Some(created) = tb_device {
                println!("  • {} → {} (TB ID: {})", 
                         local_device.name,
                         created.name,
                         created.id.as_ref().map(|id| &id.id).unwrap_or(&String::from("Unknown")));
            }
        }
    }

    Ok(())
}


#[tokio::test]
async fn test_get_device_access_token() -> Result<(), Box<dyn Error>> {
    let device_id = "4bf418f0-8f1c-11f0-9579-29cc11faaa8f".to_string();
    let base_url = "https://monitoring.avaasia.co".to_string();

    let mut client = ThingsBoardClient::new(&base_url);
    client.login("jaydenyong28@gmail.com", "lalala88").await?;
    let access_token = client.get_device_access_token(&device_id).await?;
    println!("Device Access Token for {}: {}", device_id, access_token);
    Ok(())
}
// UTe1BOQVdbEpSZx8f9Il


#[tokio::test]
async fn test_generate_device_catalog_csv() -> Result<(), Box<dyn Error>> {
    let entity_group_id = "e695a840-53e1-11f0-8c80-571814f5abd0"; // ACCV-P002-King Jade group
    let base_url = "https://monitoring.avaasia.co".to_string();
    let output_dir = "example_catalogs";

    let mut client = ThingsBoardClient::new(&base_url);
    client.login("jaydenyong28@gmail.com", "lalala88").await?;

    println!("\n{}", "=".repeat(80));
    println!("TESTING CSV DEVICE CATALOG GENERATION");
    println!("{}", "=".repeat(80));

    let result = client.generate_device_catalog_csv(entity_group_id, output_dir).await?;
    
    println!("\n{}", result);
    println!("\n{}", "=".repeat(80));
    println!("TEST COMPLETED - Check the generated CSV file in: {}", output_dir);
    println!("{}", "=".repeat(80));

    Ok(())
}


#[tokio::test]
async fn test_add_device_telemetry() -> Result<(), Box<dyn Error>> {
    let device_token = "4e327220-8fca-11f0-9579-29cc11faaa8f".to_string();
    let base_url = "https://monitoring.avaasia.co".to_string();

    let mut client = ThingsBoardClient::new(&base_url);
    client.login("jaydenyong28@gmail.com", "lalala88").await?;
    let telemetry = serde_json::json!({
        "temperature": 25.5,
        "humidity": 60,
        "status": "active"
    }); // Example telemetry data

    // Convert telemetry to HashMap<String, Value>
    let telemetry_map: std::collections::HashMap<String, serde_json::Value> = telemetry.as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    client.save_device_telemetry(&device_token, &telemetry_map).await?;
    Ok(())
}