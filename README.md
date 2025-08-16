# AVA Device Logger

A comprehensive edge device logging solution for industrial protocols including Modbus RTU, Modbus TCP, and IEC 104.

## Features

- **Multi-Protocol Support**: Modbus RTU, Modbus TCP, and IEC 104
- **Real-time Data Logging**: Continuous data collection and storage
- **Web-based Configuration**: React frontend for easy device setup
- **REST API**: Complete API for device management and data access
- **Socket.IO Integration**: Real-time data streaming to web clients
- **SQLite Database**: Lightweight local data storage
- **Edge Computing Ready**: Designed for deployment on edge devices
- **Automatic Cleanup**: Configurable log rotation and cleanup

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   React Web UI │◄───┤  Rust Backend   │◄───┤  Industrial     │
│                 │    │                 │    │  Devices        │
│ - Dashboard     │    │ - REST API      │    │                 │
│ - Configuration │    │ - Socket.IO     │    │ - Modbus RTU    │
│ - Data Logs     │    │ - Data Logging  │    │ - Modbus TCP    │
│ - System Config │    │ - SQLite DB     │    │ - IEC 104       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Quick Start

### Prerequisites

- Rust (1.70 or later)
- Node.js (16 or later) - for React frontend development
- SQLite3

### Running the Application

1. **Clone and build the Rust backend:**
```bash
cd /Users/cxyip1/Projects/ava-device-logger2
cargo build --release
```

2. **Run the backend:**
```bash
cargo run
```

The server will start on http://localhost:8080

3. **For React development (optional):**
```bash
cd web
npm install
npm start  # Runs on http://localhost:3000 with proxy to backend
```

### Configuration

The application creates a default `config.toml` file on first run. You can modify it or use the web interface to configure:

- Server settings (host, port)
- Database settings (path, cleanup intervals)
- Device configurations
- Logging settings

### Example Device Configuration

```toml
[[devices]]
id = "modbus_device_1"
name = "Temperature Sensor"
enabled = true
polling_interval_ms = 1000
timeout_ms = 5000
retry_count = 3

[devices.protocol]
type = "modbus_tcp"
host = "192.168.1.100"
port = 502
slave_id = 1

[[devices.tags]]
name = "temperature"
address = 1
data_type = "holding_register"
description = "Room temperature"

[devices.tags.scaling]
multiplier = 0.1
offset = 0.0
unit = "°C"
```

## API Endpoints

### Device Management
- `GET /api/devices` - List all devices
- `GET /api/devices/{id}` - Get device details
- `POST /api/devices` - Create new device
- `PUT /api/devices/{id}` - Update device
- `DELETE /api/devices/{id}` - Delete device
- `POST /api/devices/{id}/start` - Start device logging
- `POST /api/devices/{id}/stop` - Stop device logging

### Enhanced Device Management with Models
- `GET /api/device-models` - List all available device models
- `GET /api/device-models/{id}` - Get specific device model details  
- `GET /api/device-models/{id}/tags` - Get tag templates for a model
- `GET /api/devices-enhanced` - List devices with their tags
- `POST /api/devices-enhanced` - Create device with tags from model
- `GET /api/devices-enhanced/{id}` - Get device with all tag details
- `GET /api/devices/{id}/tags` - Get tags for a specific device

### Data Access
- `GET /api/logs` - Get all logs (with pagination)
- `GET /api/logs/{device_id}` - Get logs for specific device
- `GET /api/status` - Get system and device status

### Configuration
- `GET /api/config` - Get system configuration
- `POST /api/config` - Update system configuration

## Supported Protocols

### Modbus TCP
- Full support for reading coils, discrete inputs, holding registers, and input registers
- Configurable slave ID, host, and port
- Support for various data types (uint16, int16, uint32, int32, float32)

### Modbus RTU
- Serial communication support
- Configurable baud rate, data bits, stop bits, and parity
- Same data type support as Modbus TCP

### IEC 104
- TCP/IP communication
- Support for single-point information, measured values
- Configurable common address
- Automatic interrogation for data collection

## Data Types

The system supports the following data types:
- **Coil**: Boolean values (0/1)
- **Discrete Input**: Boolean input values
- **Holding Register**: 16-bit register values
- **Input Register**: 16-bit input values
- **UInt16**: Unsigned 16-bit integer
- **Int16**: Signed 16-bit integer
- **UInt32**: Unsigned 32-bit integer (2 registers)
- **Int32**: Signed 32-bit integer (2 registers)
- **Float32**: 32-bit floating point (2 registers)

## Scaling and Units

Each tag can have optional scaling configuration:
```toml
[scaling]
multiplier = 0.1    # Multiply raw value
offset = -10.0      # Add offset after multiplication
unit = "°C"         # Display unit
```

Final value = (raw_value * multiplier) + offset

## Database Schema

The SQLite database contains:

### log_entries
- `id`: Primary key
- `device_id`: Device identifier
- `tag_name`: Tag name
- `value`: Numeric value
- `quality`: Data quality (Good/Bad)
- `timestamp`: ISO 8601 timestamp
- `unit`: Optional unit string

### device_status
- `device_id`: Device identifier
- `status`: Current status (Connected/Error/Stopped)
- `last_update`: Last status update
- `error_message`: Optional error description
- `connection_count`: Number of connections made

## Development

### Project Structure

```
src/
├── main.rs          # Application entry point
├── config.rs        # Configuration management
├── database.rs      # SQLite database operations
├── modbus.rs        # Modbus client implementation
├── iec104.rs        # IEC 104 client implementation
├── logging.rs       # Logging service coordination
├── api.rs           # REST API handlers
└── websocket.rs     # Socket.IO handlers

web/
├── src/
│   ├── components/  # React components
│   ├── App.js       # Main React app
│   └── index.js     # React entry point
├── public/          # Static assets
└── build/           # Production build output
```

### Adding New Protocols

1. Create a new client module (e.g., `src/dnp3.rs`)
2. Implement the client with `connect()`, `### Enhanced Device Management

AVA Device Logger now includes advanced device configuration features:

#### Device Models
- **Predefined Models**: Pre-configured device models for common industrial equipment
- **Tag Templates**: Automatic tag population based on device model selection
- **Custom Devices**: Support for devices without predefined templates
- **Manufacturer Support**: Models organized by manufacturer (Schneider, Siemens, ABB, etc.)

#### Supported Device Models
- **Schneider Electric**:
  - Modicon M221 PLC (Production counters, temperature sensors, pressure sensors)
  - PowerLogic PM5000 Energy Meter (Voltage, current, power, energy measurements)
- **Siemens**:
  - S7-1200 PLC (Data blocks, analog inputs, motor control)
- **ABB**:
  - AC500 PLC (Standard industrial automation tags)
- **Generic**:
  - IEC 104 RTU (Status points, analog values, counters)
  - Custom Device (No predefined tags)

#### Tag Configuration Features
- **Automatic Population**: Tags auto-populated when selecting a device model
- **Custom Tags**: Add, edit, and remove tags as needed
- **Data Types**: Support for coils, registers, and various numeric formats
- **Scaling**: Built-in scaling with multiplier and offset
- **Units**: Unit specification for engineering values
- **Enable/Disable**: Individual tag control

#### Enhanced API Endpoints

##### Device Models
- `GET /api/device-models` - List all available device models
- `GET /api/device-models/{id}` - Get specific device model details  
- `GET /api/device-models/{id}/tags` - Get tag templates for a model

##### Enhanced Device Management
- `GET /api/devices-enhanced` - List devices with their tags
- `POST /api/devices-enhanced` - Create device with tags from model
- `GET /api/devices-enhanced/{id}` - Get device with all tag details
- `GET /api/devices/{id}/tags` - Get tags for a specific device

### Web Interface Enhancements

#### Enhanced Device Configuration
Navigate to `/devices-enhanced` to access the new device configuration interface:

- **Model Browser**: Filterable list of all available device models
- **Smart Tag Population**: Automatic tag creation based on selected model
- **Real-time Preview**: See tag templates before creating device
- **Advanced Filtering**: Filter models by protocol, manufacturer, or search text
- **Inline Tag Editing**: Edit tag properties directly in the configuration table

#### Device Model Browser
- **Search & Filter**: Find models by name, manufacturer, or protocol
- **Tag Preview**: View all predefined tags before selection
- **Model Details**: Complete model information including descriptions
- **Protocol Support**: Visual indicators for supported protocolss()`, and `disconnect()` methods
3. Add the protocol variant to `ProtocolConfig` in `config.rs`
4. Update the `DeviceClient` enum in `logging.rs`
5. Add protocol-specific UI components in the React frontend

### Building for Production

```bash
# Build React frontend
cd web
npm run build

# Build Rust backend
cd ..
cargo build --release

# The binary will be in target/release/ava-device-logger
```

## Deployment

### Systemd Service (Linux)

Create `/etc/systemd/system/ava-device-logger.service`:

```ini
[Unit]
Description=AVA Device Logger
After=network.target

[Service]
Type=simple
User=ava
WorkingDirectory=/opt/ava-device-logger
ExecStart=/opt/ava-device-logger/ava-device-logger
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable ava-device-logger
sudo systemctl start ava-device-logger
```

### Docker Deployment

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y sqlite3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ava-device-logger /usr/local/bin/
COPY web/build /app/web/build
WORKDIR /app
EXPOSE 8080
CMD ["ava-device-logger"]
```

## Configuration Examples

### Multiple Device Setup

```toml
[server]
port = 8080
host = "0.0.0.0"

[database]
path = "data.db"
max_log_entries = 1000000
cleanup_interval_hours = 24

# Modbus TCP Device
[[devices]]
id = "plc_main"
name = "Main PLC"
enabled = true
polling_interval_ms = 1000
timeout_ms = 5000
retry_count = 3

[devices.protocol]
type = "modbus_tcp"
host = "192.168.1.10"
port = 502
slave_id = 1

[[devices.tags]]
name = "production_count"
address = 100
data_type = "uint32"

[[devices.tags]]
name = "temperature"
address = 200
data_type = "float32"
[devices.tags.scaling]
multiplier = 1.0
offset = 0.0
unit = "°C"

# IEC 104 Device
[[devices]]
id = "scada_station"
name = "SCADA Station"
enabled = true
polling_interval_ms = 2000
timeout_ms = 10000
retry_count = 2

[devices.protocol]
type = "iec104"
host = "192.168.1.20"
port = 2404
common_address = 1
```

## Troubleshooting

### Common Issues

1. **Connection Failed**
   - Check network connectivity
   - Verify device IP address and port
   - Ensure firewall allows connections

2. **Permission Denied (Serial)**
   - Add user to dialout group: `sudo usermod -a -G dialout $USER`
   - Check device permissions: `ls -l /dev/ttyUSB0`

3. **Database Locked**
   - Ensure only one instance is running
   - Check file permissions on database file

4. **High Memory Usage**
   - Reduce `max_log_entries` in configuration
   - Decrease `cleanup_interval_hours` for more frequent cleanup

### Logging

Set the log level in configuration:
```toml
[logging]
level = "debug"  # error, warn, info, debug, trace
file_path = "app.log"
```

View logs:
```bash
tail -f app.log
```

## License

This project is licensed under the MIT License.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

For questions or support, please create an issue in the repository.
