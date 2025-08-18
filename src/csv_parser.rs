use crate::database::{CsvModbusTcpTagRecord, CreateModbusTcpTagRegister};
use anyhow::{Result, anyhow};
use csv::ReaderBuilder;
use std::io::Read;

pub struct ModbusTcpCsvParserService;

impl ModbusTcpCsvParserService {
    pub fn new() -> Self {
        ModbusTcpCsvParserService
    }

    pub fn parse_csv<R: Read>(&self, reader: R) -> Result<Vec<CreateModbusTcpTagRegister>> {
        let mut csv_reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);

        let mut tag_registers = Vec::new();

        for (row_index, result) in csv_reader.deserialize().enumerate() {
            let record: CsvModbusTcpTagRecord = result
                .map_err(|e| anyhow!("CSV parsing error at row {}: {}", row_index + 2, e))?;
            
            let tag_register = self.convert_csv_record_to_create_tag_register(record, row_index + 2)?;
            tag_registers.push(tag_register);
        }

        Ok(tag_registers)
    }

    pub fn parse_csv_with_device_model<R: Read>(&self, reader: R, device_model_name: &str) -> Result<Vec<CreateModbusTcpTagRegister>> {
        let mut csv_reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);

        let mut tag_registers = Vec::new();

        for (row_index, result) in csv_reader.deserialize().enumerate() {
            let record: CsvModbusTcpTagRecord = result
                .map_err(|e| anyhow!("CSV parsing error at row {}: {}", row_index + 2, e))?;
            
            let tag_register = self.convert_csv_record_to_create_tag_register_with_device_model(record, device_model_name, row_index + 2)?;
            tag_registers.push(tag_register);
        }

        Ok(tag_registers)
    }

    pub fn parse_csv_with_device_model_and_manufacturer<R: Read>(&self, reader: R, device_model_name: &str, manufacturer: &str) -> Result<Vec<CreateModbusTcpTagRegister>> {
        println!("DEBUG: parse_csv_with_device_model_and_manufacturer called with device_model_name: {} and manufacturer: {}", device_model_name, manufacturer);
        
        let mut csv_reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);

        let mut tag_registers = Vec::new();

        for (row_index, result) in csv_reader.deserialize().enumerate() {
            let record: CsvModbusTcpTagRecord = result
                .map_err(|e| anyhow!("CSV parsing error at row {}: {}", row_index + 2, e))?;
            
            let tag_register = self.convert_csv_record_to_create_tag_register_with_device_model_and_manufacturer(record, device_model_name, manufacturer)?;
            tag_registers.push(tag_register);
        }

        Ok(tag_registers)
    }

    fn convert_csv_record_to_create_tag_register(
        &self, 
        record: CsvModbusTcpTagRecord, 
        row_number: usize
    ) -> Result<CreateModbusTcpTagRegister> {
        Ok(CreateModbusTcpTagRegister {
            device_brand: record.device_brand.trim().to_string(),
            device_model: record.device_model.trim().to_string(),
            ava_type: record.ava_type.trim().to_string(),
            mppt: self.parse_optional_int_from_string(&record.mppt, "MPPT", row_number)?,
            input: self.parse_optional_int_from_string(&record.input, "INPUT", row_number)?,
            data_label: record.data_label.trim().to_string(),
            address: record.address,
            size: record.size,
            modbus_type: record.modbus_type.trim().to_string(),
            divider: record.divider,
            register_type: record.register_type.trim().to_string(),
        })
    }

    fn convert_csv_record_to_create_tag_register_with_device_model(
        &self, 
        record: CsvModbusTcpTagRecord, 
        device_model_name: &str,
        row_number: usize
    ) -> Result<CreateModbusTcpTagRegister> {
        Ok(CreateModbusTcpTagRegister {
            device_brand: record.device_brand.trim().to_string(),
            device_model: device_model_name.to_string(), // Use provided device model name instead of CSV column
            ava_type: record.ava_type.trim().to_string(),
            mppt: self.parse_optional_int_from_string(&record.mppt, "MPPT", row_number)?,
            input: self.parse_optional_int_from_string(&record.input, "INPUT", row_number)?,
            data_label: record.data_label.trim().to_string(),
            address: record.address,
            size: record.size,
            modbus_type: record.modbus_type.trim().to_string(),
            divider: record.divider,
            register_type: record.register_type.trim().to_string(),
        })
    }

    fn parse_optional_int_from_string(
        &self, 
        value: &str, 
        field_name: &str, 
        row_number: usize
    ) -> Result<Option<i32>> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Ok(None) // Empty means inverter-level register
        } else {
            trimmed.parse::<i32>()
                .map(Some)
                .map_err(|e| anyhow!("Row {}: Failed to parse {} '{}': {}", row_number, field_name, trimmed, e))
        }
    }

    pub fn validate_csv_headers<R: Read>(&self, reader: R) -> Result<()> {
        let mut csv_reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);

        let headers = csv_reader.headers()
            .map_err(|e| anyhow!("Failed to read CSV headers: {}", e))?;
            
        let expected_headers = vec![
            "Device Brand", "Device Model", "AVA Type", "MPPT", "INPUT",
            "Data Label", "Address", "Size", "Modbus Type", "Divider", "Register Type"
        ];

        for expected_header in expected_headers {
            if !headers.iter().any(|h| h.trim() == expected_header) {
                return Err(anyhow!("Missing required header: '{}'", expected_header));
            }
        }

        Ok(())
    }

    pub fn validate_record_data(&self, records: &[CreateModbusTcpTagRegister]) -> Result<()> {
        for (index, record) in records.iter().enumerate() {
            let row_number = index + 2; // CSV row number (header is row 1)

            // Validate required fields
            if record.device_brand.is_empty() {
                return Err(anyhow!("Row {}: Device Brand cannot be empty", row_number));
            }
            if record.device_model.is_empty() {
                return Err(anyhow!("Row {}: Device Model cannot be empty", row_number));
            }
            if record.ava_type.is_empty() {
                return Err(anyhow!("Row {}: AVA Type cannot be empty", row_number));
            }
            if record.data_label.is_empty() {
                return Err(anyhow!("Row {}: Data Label cannot be empty", row_number));
            }

            // Validate AVA Type
            let valid_ava_types = ["Inverter", "String", "MPPT", "Battery", "Meter", "Weather Station", "PowerMeter", "Plant"];
            if !valid_ava_types.contains(&record.ava_type.as_str()) {
                return Err(anyhow!("Row {}: Invalid AVA Type '{}'. Valid types: {:?}", 
                    row_number, record.ava_type, valid_ava_types));
            }

            // Validate modbus type
            let valid_modbus_types = ["U16", "I16", "U32", "I32", "FLOAT", "DOUBLE", "F32"];
            if !valid_modbus_types.contains(&record.modbus_type.as_str()) {
                return Err(anyhow!("Row {}: Invalid Modbus Type '{}'. Valid types: {:?}", 
                    row_number, record.modbus_type, valid_modbus_types));
            }

            // Validate register type
            let valid_register_types = ["input", "holding", "coil", "discrete"];
            if !valid_register_types.contains(&record.register_type.as_str()) {
                return Err(anyhow!("Row {}: Invalid Register Type '{}'. Valid types: {:?}", 
                    row_number, record.register_type, valid_register_types));
            }

            // Validate size based on modbus type
            let expected_size = match record.modbus_type.as_str() {
                "U16" | "I16" => 1,
                "U32" | "I32" | "FLOAT" => 2,
                "DOUBLE" => 4,
                _ => continue,
            };
            if record.size != expected_size {
                return Err(anyhow!("Row {}: Size {} doesn't match Modbus Type {}. Expected size: {}", 
                    row_number, record.size, record.modbus_type, expected_size));
            }

            // Validate address range
            if record.address < 0 || record.address > 65535 {
                return Err(anyhow!("Row {}: Address {} is out of valid range (0-65535)", 
                    row_number, record.address));
            }

            // Validate MPPT and INPUT logic
            match record.ava_type.as_str() {
                "Inverter" => {
                    // Inverter-level registers should not have MPPT or INPUT
                    if record.mppt.is_some() || record.input.is_some() {
                        return Err(anyhow!("Row {}: Inverter-level registers should not have MPPT or INPUT values", 
                            row_number));
                    }
                }
                "String" => {
                    // String-level registers should have both MPPT and INPUT
                    if record.mppt.is_none() || record.input.is_none() {
                        return Err(anyhow!("Row {}: String-level registers must have both MPPT and INPUT values", 
                            row_number));
                    }
                    
                    // Validate MPPT and INPUT ranges
                    if let Some(mppt) = record.mppt {
                        if mppt < 1 || mppt > 20 {
                            return Err(anyhow!("Row {}: MPPT {} is out of valid range (1-20)", 
                                row_number, mppt));
                        }
                    }
                    if let Some(input) = record.input {
                        if input < 1 || input > 50 {
                            return Err(anyhow!("Row {}: INPUT {} is out of valid range (1-50)", 
                                row_number, input));
                        }
                    }
                }
                _ => {
                    // Other types (MPPT, Battery, Meter) - flexible rules
                }
            }

            // Validate divider
            if record.divider <= 0.0 {
                return Err(anyhow!("Row {}: Divider must be greater than 0", row_number));
            }
        }

        Ok(())
    }

    fn convert_csv_record_to_create_tag_register_with_device_model_and_manufacturer(
        &self,
        record: CsvModbusTcpTagRecord,
        device_model_name: &str,
        manufacturer: &str,
    ) -> Result<CreateModbusTcpTagRegister> {
        println!("DEBUG: Converting record with device_model_name: {} and manufacturer: {} (CSV had device_brand: {} and device_model: {})", 
                 device_model_name, manufacturer, record.device_brand, record.device_model);
        Ok(CreateModbusTcpTagRegister {
            device_brand: manufacturer.to_string(), // Use manufacturer instead of CSV device_brand
            device_model: device_model_name.to_string(), // Use device_model_name instead of CSV device_model
            ava_type: record.ava_type.trim().to_string(),
            mppt: self.parse_optional_int_from_string(&record.mppt, "MPPT", 0)?,
            input: self.parse_optional_int_from_string(&record.input, "INPUT", 0)?,
            data_label: record.data_label.trim().to_string(),
            address: record.address,
            size: record.size,
            modbus_type: record.modbus_type.trim().to_string(),
            divider: record.divider,
            register_type: record.register_type.trim().to_string(),
        })
    }

    pub fn get_summary(&self, records: &[CreateModbusTcpTagRegister]) -> String {
        let total_count = records.len();
        let inverter_count = records.iter().filter(|r| r.ava_type == "Inverter").count();
        let string_count = records.iter().filter(|r| r.ava_type == "String").count();
        let input_registers = records.iter().filter(|r| r.register_type == "input").count();
        let holding_registers = records.iter().filter(|r| r.register_type == "holding").count();

        format!(
            "Total: {} records | Inverter: {} | String: {} | Input registers: {} | Holding registers: {}",
            total_count, inverter_count, string_count, input_registers, holding_registers
        )
    }
}
