# ZephyrFS CLI

Command-line interface for ZephyrFS distributed P2P storage network.

## Features

- =€ **Easy Setup**: Initialize and configure nodes with interactive prompts
- = **Network Management**: Join and manage P2P network connections
- =Á **File Operations**: Upload, download, and list files across the network
- =Ê **Monitoring**: Built-in metrics and status monitoring
- ¡ **Performance**: Benchmarking and performance analysis tools
- =' **Configuration**: YAML/TOML configuration file support

## Installation

### From Source

```bash
git clone https://github.com/ZephyrFS/zephyrfs-cli.git
cd zephyrfs-cli
cargo build --release
```

The compiled binary will be available at `target/release/zephyrfs`.

### Using Cargo

```bash
cargo install zephyrfs-cli
```

## Quick Start

### 1. Initialize a Node

```bash
# Interactive setup
zephyrfs init

# Non-interactive with options
zephyrfs init --name my-node --port 8080 --storage 50
```

### 2. Join a Network

```bash
zephyrfs join 192.168.1.100:8081
```

### 3. Upload Files

```bash
zephyrfs upload /path/to/file.txt
zephyrfs upload large-file.zip --verify
```

### 4. List Files

```bash
zephyrfs list
zephyrfs list --long --sort size
```

### 5. Download Files

```bash
zephyrfs download a1b2c3d4e5f6... --output downloaded-file.txt
```

### 6. Check Status

```bash
zephyrfs status
zephyrfs status --detailed --watch 5
```

## Commands

### `init` - Initialize Node

Initialize a new ZephyrFS node with configuration.

```bash
zephyrfs init [OPTIONS]

Options:
  -n, --name <NAME>        Node name (optional)
  -d, --data-dir <DIR>     Data directory path
  -p, --port <PORT>        Listen port for the node
  -s, --storage <GB>       Maximum storage allocation in GB
      --no-interactive     Skip interactive configuration
```

### `join` - Join Network

Connect to an existing ZephyrFS network.

```bash
zephyrfs join <BOOTSTRAP_PEER> [OPTIONS]

Arguments:
  <BOOTSTRAP_PEER>    Bootstrap peer address (host:port)

Options:
  -t, --timeout <SECS>    Timeout in seconds [default: 30]
      --skip-test         Skip connectivity test
```

### `upload` - Upload Files

Upload files to the distributed network.

```bash
zephyrfs upload <FILE> [OPTIONS]

Arguments:
  <FILE>    File path to upload

Options:
  -n, --name <NAME>      Custom file name
      --progress         Show progress bar [default: true]
      --verify           Verify upload after completion
```

### `download` - Download Files

Download files from the network by hash.

```bash
zephyrfs download <FILE_HASH> [OPTIONS]

Arguments:
  <FILE_HASH>    File hash to download

Options:
  -o, --output <PATH>    Output file path
      --force            Overwrite existing file
      --progress         Show progress bar [default: true]
      --verify           Verify download integrity [default: true]
```

### `list` - List Files

List files available in the network.

```bash
zephyrfs list [OPTIONS]

Options:
  -l, --long                Show detailed information
  -s, --sort <COLUMN>       Sort by column (name, size, date, chunks) [default: name]
  -r, --reverse             Reverse sort order
  -f, --filter <PATTERN>    Filter by name pattern
      --format <FORMAT>     Output format (table, json, csv) [default: table]
```

### `status` - Node Status

Show node status and network information.

```bash
zephyrfs status [OPTIONS]

Options:
  -d, --detailed           Show detailed status information
      --format <FORMAT>    Output format (table, json) [default: table]
  -r, --watch <SECS>       Refresh interval in seconds (0 for single check) [default: 0]
```

## Configuration

ZephyrFS CLI uses a configuration file to store node settings. The default location is:
- Linux/macOS: `~/.config/zephyrfs/config.yaml`
- Windows: `%APPDATA%\\zephyrfs\\config.yaml`

### Configuration File Format

```yaml
node:
  id: null
  name: "my-zephyr-node"
  data_dir: "~/.zephyrfs"
  listen_port: 8080

network:
  bootstrap_peers:
    - "127.0.0.1:8081"
    - "127.0.0.1:8082"
  max_connections: 50
  connection_timeout: 30

storage:
  max_storage_gb: 10
  chunk_size_mb: 1
  replication_factor: 3

coordinator:
  endpoint: "http://127.0.0.1:9000"
  timeout: 30
  retry_attempts: 3
```

### TOML Configuration

You can also use TOML format by saving as `config.toml`:

```toml
[node]
name = "my-zephyr-node"
data_dir = "~/.zephyrfs"
listen_port = 8080

[network]
bootstrap_peers = ["127.0.0.1:8081", "127.0.0.1:8082"]
max_connections = 50
connection_timeout = 30

[storage]
max_storage_gb = 10
chunk_size_mb = 1
replication_factor = 3

[coordinator]
endpoint = "http://127.0.0.1:9000"
timeout = 30
retry_attempts = 3
```

## Examples

### Basic File Operations

```bash
# Initialize node with 100GB storage
zephyrfs init --storage 100 --name production-node

# Join production network
zephyrfs join prod.zephyrfs.net:8081

# Upload important files
zephyrfs upload documents.tar.gz --verify
zephyrfs upload photos/ --name family-photos

# List all files with details
zephyrfs list --long --sort date --reverse

# Download specific file
zephyrfs download abc123... --output restored-documents.tar.gz
```

### Monitoring and Maintenance

```bash
# Watch node status in real-time
zephyrfs status --detailed --watch 10

# Export metrics for analysis
zephyrfs status --format json > node-metrics.json

# Check network connectivity
zephyrfs join test.zephyrfs.net:8081 --skip-test false
```

### Batch Operations

```bash
# Upload multiple files
for file in /data/*.backup; do
    zephyrfs upload "$file" --verify
done

# List files in CSV format for processing
zephyrfs list --format csv > file-inventory.csv
```

## Performance

### Benchmarking

Run performance benchmarks:

```bash
cargo bench
```

This will generate HTML reports in `target/criterion/` with detailed performance analysis.

### Metrics

The CLI automatically collects performance metrics including:
- Command execution times
- Network request latencies
- File transfer speeds
- Memory and CPU usage
- Cache hit rates

View current metrics:
```bash
zephyrfs status --detailed --format json
```

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
cargo test --test integration_tests
```

Note: Some integration tests require running ZephyrFS nodes and are marked with `#[ignore]`. To run them:

```bash
cargo test --test integration_tests -- --ignored
```

## Troubleshooting

### Common Issues

**Connection Refused**
```
Error: Failed to connect to node
```
- Ensure the ZephyrFS node is running
- Check the listen port in configuration
- Verify firewall settings

**File Not Found**
```
Error: File not found: abc123...
```
- File may have been removed from the network
- Check file hash is correct
- Try listing files to see what's available

**Configuration Error**
```
Error: Failed to parse YAML config
```
- Validate YAML/TOML syntax
- Check file permissions
- Use `--config` flag to specify custom location

### Debug Mode

Enable verbose logging:

```bash
zephyrfs --verbose <command>
```

### Log Files

Check logs in the data directory:
- `~/.zephyrfs/logs/zephyrfs-cli.log`

## Development

### Building from Source

```bash
git clone https://github.com/ZephyrFS/zephyrfs-cli.git
cd zephyrfs-cli
cargo build
```

### Running Tests

```bash
cargo test --all-features
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run tests and benchmarks
6. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Related Projects

- [zephyrfs-node](https://github.com/ZephyrFS/zephyrfs-node) - Core P2P storage node
- [zephyrfs-coordinator](https://github.com/ZephyrFS/zephyrfs-coordinator) - Network coordination service
- [zephyrfs-proto](https://github.com/ZephyrFS/zephyrfs-proto) - Protocol definitions
- [zephyrfs-deploy](https://github.com/ZephyrFS/zephyrfs-deploy) - Deployment configurations