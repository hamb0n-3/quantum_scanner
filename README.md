# Quantum Scanner (Rust Edition)

An advanced port scanner with evasion capabilities written in Rust.

## Features

- **Multiple Scan Techniques**: SYN, ACK, FIN, XMAS, NULL, SSL, UDP, and more
- **Stealthy Evasion**: Fragmentation, protocol mimicry, TLS echo scans
- **Enhanced Security**: Memory-only operation, RAM disk support, secure cleanup
- **Advanced Evasion**: OS fingerprint spoofing, TTL jittering, protocol mimicry
- **Tor Integration**: Optional traffic routing through Tor (when available)
- **Performance**: High-speed concurrent scanning leveraging Rust's async capabilities
- **Service Detection**: Identifies services running on open ports
- **SSL Analysis**: Examines SSL/TLS certificates and configuration
- **Cross-Platform**: Works on Linux, macOS, and Windows

## Security Notice

This tool is designed for network security professionals conducting authorized security tests. 
Running port scans against networks or systems without explicit permission is illegal in many jurisdictions and violates most network use policies.

**You are responsible for using this tool ethically and legally.**

## Installation

### Prerequisites

- Rust 1.67.0 or later
- Cargo
- libpcap development files (for packet capture capabilities)

On Debian/Ubuntu systems:
```
sudo apt install libpcap-dev
```

On RHEL/Fedora:
```
sudo dnf install libpcap-devel
```

On macOS with Homebrew:
```
brew install libpcap
```

### Building from source

```
git clone https://github.com/yourusername/quantum_scanner_rs.git
cd quantum_scanner_rs
./build.sh
```

For additional build options:
```
./build.sh --help
```

To install the binary system-wide (requires root):
```
sudo ./build.sh --install
```

The compiled binary will be in `target/release/quantum_scanner`.

## Usage

Basic syntax:
```
quantum_scanner [OPTIONS] <TARGET>
```

Examples:
```
# Simple SYN scan of common ports on a single host
quantum_scanner 192.168.1.1

# Comprehensive scan of a host with multiple techniques
quantum_scanner --scan-types syn,fin,ssl,udp --ports 1-1000 192.168.1.1

# Stealthy scan with evasion techniques
quantum_scanner --evasion 192.168.1.1

# Scan with protocol mimicry
quantum_scanner --scan-types mimic --mimic-protocol HTTP 192.168.1.1

# Scan an entire subnet
quantum_scanner --scan-types syn --ports 22,80,443 192.168.1.0/24

# Enable disk mode to write logs to disk (memory-only is default)
quantum_scanner -d 192.168.1.1
```

## Command-line Options

Basic options:
```
<TARGET>                Target IP address, hostname, or CIDR notation for subnet
-p, --ports             Ports to scan (comma-separated, ranges like 1-1000) [default: 1-1000]
-s, --scan-types        Scan techniques to use [default: syn]
                        [possible values: syn, ack, fin, xmas, null, window, ssl, udp, tls_echo, mimic, frag]
-c, --concurrency       Maximum concurrent operations [default: 100]
-o, --output            Write results to file
-j, --json              Output results in JSON format
-v, --verbose           Enable verbose output
-d, --disk-mode         Enable disk mode (writes logs and data to disk)
-h, --help              Print help information
-V, --version           Print version information
```

Evasion options:
```
-e, --evasion           Enable evasion techniques
--mimic-protocol        Protocol to mimic in mimic scans [default: HTTP]
--mimic-os              Operating system to mimic (windows, linux, macos, random)
--ttl-jitter            TTL jitter amount for enhanced evasion (1-5) [default: 2]
```

Performance options:
```
-r, --rate              Maximum packets per second (0 for automatic rate) [default: 0]
-t, --timeout           Scan timeout in seconds [default: 3.0]
```

## Enhanced Security Features

### Memory-Only Operation

By default, Quantum Scanner operates in memory-only mode to avoid leaving sensitive data on disk:

```
quantum_scanner target.com
```

To enable disk mode and write logs to disk:

```
quantum_scanner -d target.com
```

## License

MIT License

## Acknowledgments

This is a Rust port of the original Python Quantum Scanner. 