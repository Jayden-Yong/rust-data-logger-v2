# AVA Device Logger

A comprehensive edge device logging solution for industrial protocols including Modbus RTU, Modbus TCP, and IEC 104. Features role-based access control with installer and admin workflows, ThingsBoard cloud integration, and advanced device configuration management.

## Features

- **Multi-Protocol Support**: Modbus RTU, Modbus TCP, and IEC 104
- **Real-time Data Logging**: Continuous data collection and storage
- **Role-Based Access Control**: Separate installer and admin accounts with different permissions
- **Web-based Configuration**: React frontend for easy device setup
- **ThingsBoard Integration**: Automatic device sync and attribute management
- **Enhanced Device Management**: Advanced device configuration with model templates and tag management
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

### Initial Setup

The application automatically creates default user accounts on first run:

**Admin Account**
- Username: `admin`
- Password: `admin123`
- Permissions: Full access to all features including ThingsBoard sync, plant configuration, and system settings

**Installer Account**
- Username: `installer`
- Password: `installer123`
- Permissions: Device configuration and management only

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

3. **Access the web interface:**
- Navigate to http://localhost:3000
- Login with either admin or installer credentials
- Installers will be directed to device configuration
- Admins have access to all features including plant setup

4. **For React development (optional):**
```bash
cd web
npm install
npm start  # Runs on http://localhost:3000 with proxy to backend
```

### Admin Setup Workflow

Before installers can use the system, admins must complete the plant configuration:

1. **Login as admin** (username: `admin`, password: `admin123`)
2. **Navigate to Plant Config** from the sidebar
3. **Select ThingsBoard Entity Group** - Choose the plant's device group from ThingsBoard
4. **Save Configuration** - This sets up the plant context for all devices

Once plant configuration is complete, installers can create and manage devices that will be associated with the configured plant.

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

### Authentication
- `POST /api/login` - Authenticate user and create session
- `POST /api/logout` - Revoke session and logout
- `GET /api/session` - Verify current session

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
- `PUT /api/devices-enhanced/{id}` - Update device with tags
- `GET /api/devices-enhanced/{id}` - Get device with all tag details
- `GET /api/devices/{id}/tags` - Get tags for a specific device

### ThingsBoard Integration (Admin Only)
- `GET /api/thingsboard/entity-groups` - List ThingsBoard device groups
- `POST /api/sync-devices-to-thingsboard` - Sync local devices to ThingsBoard
- `POST /api/generate-device-catalog` - Generate device catalog CSV
- `GET /api/catalog-files` - List generated catalog files
- `GET /api/catalog-files/{filename}` - Download catalog file
- `DELETE /api/catalog-files/{filename}` - Delete catalog file

### Plant Configuration (Admin Only)
- `GET /api/plant-config` - Get current plant configuration
- `POST /api/plant-config` - Update plant configuration

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

## User Roles and Permissions

The system implements role-based access control with two user types:

### Installer Role
**Capabilities:**
- Device configuration and management
- Add, edit, and delete devices
- Configure device tags and polling schedules
- View device status and logs
- Access Enhanced Device Config interface

**Restrictions:**
- Cannot access ThingsBoard synchronization features
- Cannot configure plant settings
- Cannot access system administration features
- Cannot modify user accounts

### Admin Role
**Full Access Including:**
- All installer capabilities
- Plant configuration with ThingsBoard entity groups
- Device synchronization to ThingsBoard cloud
- ThingsBoard attribute management
- Device catalog generation and file management
- System configuration and settings
- User management (future feature)

### Authentication Flow
1. User navigates to the web interface
2. Login page presents username/password form
3. System validates credentials against local database
4. Session token generated with 24-hour expiration
5. Token stored in browser localStorage
6. All API requests include Authorization header with Bearer token
7. Server validates token before processing requests
8. Unauthorized requests return 401 status

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

### local_users
- `id`: Primary key
- `username`: Unique username
- `password_hash`: Bcrypt hashed password
- `role`: User role (admin or installer)
- `created_at`: Account creation timestamp

### sessions
- `id`: Primary key
- `user_id`: Foreign key to local_users
- `session_token`: Unique session identifier
- `created_at`: Session creation timestamp
- `expires_at`: Session expiration timestamp

### devices
- `id`: Device identifier
- `name`: Device name
- `serial_no`: Device serial number
- `model_id`: Foreign key to device_models
- `enabled`: Device enabled status
- `tb_device_id`: ThingsBoard device ID (NULL until synced)
- `tb_group_id`: ThingsBoard entity group ID
- `protocol_config`: JSON protocol configuration
- `polling_interval_ms`: Polling interval
- `timeout_ms`: Communication timeout
- `retry_count`: Retry attempts
- `created_at`: Creation timestamp
- `updated_at`: Last update timestamp

### device_tags
- `id`: Primary key
- `device_id`: Foreign key to devices
- `name`: Tag name
- `address`: Register/coil address
- `data_type`: Data type (coil, register, uint16, etc.)
- `size`: Number of registers
- `unit`: Measurement unit
- `scaling_multiplier`: Scale factor
- `scaling_offset`: Offset value
- `schedule_group_id`: Polling schedule group
- `agg_to_field`: ThingsBoard aggregation field
- `enabled`: Tag enabled status

### device_models
- `id`: Model identifier
- `name`: Model name
- `manufacturer`: Manufacturer name
- `protocol_type`: Supported protocol
- `description`: Model description

### tag_templates
- `id`: Template identifier
- `model_id`: Foreign key to device_models
- `name`: Tag name
- `address`: Default address
- `data_type`: Default data type
- `description`: Tag description

### plant_config
- `id`: Primary key
- `plant_name`: Plant name from ThingsBoard
- `thingsboard_entity_group_id`: Selected entity group ID
- `created_at`: Configuration timestamp
- `updated_at`: Last update timestamp

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
2. Implement the client with `connect()`, `read_tags()`, and `disconnect()` methods
3. Add the protocol variant to `ProtocolConfig` in `config.rs`
4. Update the `DeviceClient` enum in `logging.rs`
5. Add protocol-specific UI components in the React frontend

## Enhanced Device Management

AVA Device Logger includes advanced device configuration features through the Enhanced Device Config interface:

#### Key Features

**Device Models & Templates**
- Pre-configured device models for common industrial equipment organized by manufacturer
- Automatic tag population based on selected device model
- Support for custom devices without predefined templates
- Model browser with search and filtering capabilities

**Intelligent Tag Configuration**
- Tags automatically populated when selecting a device model
- Individual tag enable/disable control
- Data type support: coils, registers, and various numeric formats (UInt16, Int16, UInt32, Int32, Float32)
- Built-in scaling with multiplier and offset for engineering units
- Custom unit specification for measurements
- Address and register size configuration

**Schedule Groups**
- Tags organized by polling frequency (high, medium, low)
- Different schedule groups for different data criticality
- Visual indicators for polling intervals
- Flexible assignment of tags to schedule groups

**Field Aggregation**
- Map tags to standardized ThingsBoard fields (ia, ib, ic, frequency, pf, ua, ub, uc, etc.)
- Enables consistent data structure across different device types
- Supports aggregation for inverter and meter devices

**Device Management Table**
- Comprehensive device overview with serial numbers, models, and protocols
- Real-time status indicators (enabled/disabled)
- Tag count display per device
- Quick edit and delete actions
- Last updated timestamp tracking

#### Supported Device Models

**Schneider Electric**
- Modicon M221 PLC - Production counters, temperature sensors, pressure sensors
- PowerLogic PM5000 Energy Meter - Voltage, current, power, energy measurements

**Siemens**
- S7-1200 PLC - Data blocks, analog inputs, motor control

**ABB**
- AC500 PLC - Standard industrial automation tags

**Sungrow** (Solar Inverters)
- SG20RT, SG125CX, SG150CX - Inverter telemetry with grid and PV measurements

**Generic**
- IEC 104 RTU - Status points, analog values, counters
- Custom Device - No predefined tags, full manual configuration

#### Enhanced Device Config Workflow

1. **Access Interface**: Navigate to "Enhanced Device Config" from the sidebar
2. **Select Device Model**: Use the Model Browser to find and select appropriate device model
3. **Review Tag Templates**: Preview all predefined tags before device creation
4. **Configure Device**: Fill in device details (name, serial number, protocol settings)
5. **Customize Tags**: Enable/disable tags, adjust addresses, modify scaling as needed
6. **Assign Schedule Groups**: Set polling intervals for different tag groups
7. **Save Device**: Create device with all configured tags in one operation

#### Tag Configuration Fields

Each tag can be configured with:
- **Name**: Tag identifier
- **Address**: Register or coil address in device
- **Size**: Number of registers (1 for 16-bit, 2 for 32-bit values)
- **Data Type**: Coil, Discrete Input, Holding Register, Input Register, or numeric types
- **Unit**: Measurement unit (e.g., "V", "A", "kW", "°C")
- **Scale Multiplier**: Multiply raw value for engineering units
- **Scale Offset**: Add offset after multiplication
- **Schedule Group**: Polling frequency group
- **Agg To Field**: ThingsBoard field mapping for data aggregation
- **Enabled**: Individual tag enable/disable toggle

#### ThingsBoard Integration Features (Admin Only)

**Plant Configuration**
- Configure ThingsBoard entity group for device synchronization
- Set plant name from selected entity group
- Required before device synchronization

**Device Synchronization**
- One-click sync of all local devices to ThingsBoard cloud
- Automatic device creation with proper naming conventions
- Serial number and attribute synchronization
- Hierarchical device creation for inverters (MPPT and String devices)
- Real-time sync progress tracking with success/failure counts

**Device Attributes**
- Automatic attribute updates on device creation
- Serial number change detection and automatic ThingsBoard sync
- Device-specific attributes (INV index, customer code, manufacturer, model)
- Server-side attribute storage for metadata management

**Catalog Generation**
- Generate CSV catalogs of all devices with ThingsBoard IDs and tokens
- Download generated catalogs for external tools
- File management interface for catalog downloads and cleanup
- Includes device hierarchy (main devices, MPPT, String devices)

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

5. **Login Failed**
   - Verify username and password are correct
   - Check database for user accounts: `sqlite3 data.db "SELECT username, role FROM local_users;"`
   - Ensure default users were created on first run

6. **Session Expired**
   - Sessions expire after 24 hours
   - Re-login to generate new session token
   - Check system time is synchronized

7. **ThingsBoard Sync Failed (Admin Only)**
   - Verify plant configuration is set
   - Check ThingsBoard credentials in source code
   - Ensure entity group exists in ThingsBoard
   - Review server logs for detailed error messages

8. **Tags Not Loading from Model**
   - Ensure device is enabled before selecting model
   - Check device model has tag templates in database
   - Verify schedule groups are loaded
   - Review browser console for errors

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

## Security Considerations

### Default Credentials
The system ships with default credentials that **MUST** be changed in production:
- Admin: `admin` / `admin123`
- Installer: `installer` / `installer123`

### Password Security
- Passwords are hashed using bcrypt with cost factor 12
- Session tokens are UUID v4 with 24-hour expiration
- Sessions stored in database with automatic cleanup

### Network Security
- Run behind reverse proxy (nginx, Apache) with HTTPS
- Configure firewall to restrict access to port 8080
- Use VPN for remote access in production environments

### API Authentication
- All API endpoints (except `/api/login`) require authentication
- Authorization header with Bearer token format
- Tokens validated against database on each request
- Expired sessions automatically rejected

### Role-Based Access
- Installer role restricted to device management only
- Admin role required for ThingsBoard integration and system config
- Permissions enforced at API level with middleware
- UI elements hidden based on role but backed by server-side checks

### Best Practices
- Change default passwords immediately after deployment
- Regularly review user accounts and sessions
- Enable HTTPS in production deployments
- Keep system and dependencies updated
- Monitor logs for unauthorized access attempts
- Implement network segmentation for industrial protocols

## License

This project is licensed under the MIT License.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

For questions or support, please create an issue in the repository.
