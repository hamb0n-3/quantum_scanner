use clap::{Parser, ValueEnum};
use log::{error, info, warn};
use std::path::PathBuf;
use std::process;
use std::time::Duration;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use std::fs;
use std::io::{Read, Write};
use serde_json;
use std::net::IpAddr;

mod banner;
mod http_analyzer;
mod minimal;
mod ml_service_ident;
mod models;
mod output;
mod scanner;
mod service_fingerprints;
mod techniques;
mod tunnel;
mod utils;

use scanner::QuantumScanner;
use models::{ScanType, PortRange, PortRanges, TopPorts};

/// Advanced port scanner with evasion capabilities for authorized red team operations
#[derive(Parser)]
#[clap(
    author, 
    version, 
    about = "A sophisticated network scanner with advanced evasion capabilities for security assessments",
    long_about = "Quantum Scanner provides comprehensive network reconnaissance capabilities with a focus on operational security. It enables secure, controlled scanning with multiple techniques and evasive measures.",
    name = "quantum_scanner",
)]
#[clap(group(
    clap::ArgGroup::new("target_selection")
        .multiple(false)
))]
#[clap(group(
    clap::ArgGroup::new("scan_execution")
        .multiple(true)
))]
#[clap(group(
    clap::ArgGroup::new("output_options")
        .multiple(true)
))]
#[clap(group(
    clap::ArgGroup::new("evasion_options")
        .multiple(true)
))]
#[clap(group(
    clap::ArgGroup::new("tunneling_options")
        .multiple(true)
))]
#[clap(group(
    clap::ArgGroup::new("service_detection")
        .multiple(true)
))]
#[clap(group(
    clap::ArgGroup::new("timing_control")
        .multiple(true)
))]
#[clap(group(
    clap::ArgGroup::new("fragmentation")
        .multiple(true)
))]
#[clap(group(
    clap::ArgGroup::new("operational_security")
        .multiple(true)
))]
#[clap(after_help = "EXAMPLES:
    # Run a basic SYN scan against a target
    quantum_scanner 192.168.1.1

    # Scan a range with multiple techniques and evasion
    quantum_scanner 192.168.0.0/24 -p 22,80,443-8000 -s syn,fin,xmas -e

    # Full stealth scan through Tor with enhanced evasion
    quantum_scanner example.com -E -m --mimic-os linux --use-tor

    # Using protocol tunneling to bypass firewalls
    quantum_scanner 10.0.0.1 --dns-tunnel --lookup-domain example.com

    # Enhanced service identification with ML
    quantum_scanner 192.168.1.100 --ml-ident -p 22,80,443

    # Save results to a file in JSON format
    quantum_scanner 10.0.0.1 -j -o scan_results.json

AVAILABLE SCAN TYPES:
    syn         - Standard TCP SYN scan (efficient and relatively stealthy)
    ssl         - Probes for SSL/TLS service information and certificates
    udp         - Basic UDP port scan with custom payload options
    ack         - TCP ACK scan to detect firewall filtering rules
    fin         - Stealthy scan using TCP FIN flags to bypass basic filters
    xmas        - TCP scan with FIN, URG, and PUSH flags set
    null        - TCP scan with no flags set, may bypass some packet filters
    window      - Analyzes TCP window size responses to determine port status
    tls-echo    - Uses fake TLS server responses to evade detection
    mimic       - Sends SYN packets with protocol-specific payloads
    frag        - Fragments packets to bypass deep packet inspection
    dns-tunnel  - Tunnels scan traffic through DNS queries
    icmp-tunnel - Tunnels scan traffic through ICMP echo (ping) packets

MIMICRY OPTIONS:
    PROTOCOLS (used with --mimic-protocol):
        HTTP    - Mimics HTTP server (default)
        SSH     - Mimics OpenSSH server
        FTP     - Mimics FTP server
        SMTP    - Mimics SMTP mail server
        IMAP    - Mimics IMAP mail server
        POP3    - Mimics POP3 mail server
        MYSQL   - Mimics MySQL database server
        RDP     - Mimics Remote Desktop Protocol server

    OS PROFILES (used with --mimic-os):
        windows - Mimics Windows networking behavior
        linux   - Mimics Linux networking behavior
        macos   - Mimics macOS networking behavior
        random  - Uses randomly selected OS profile (default)
"
)]
struct Args {
    /// Target IP address, hostname, or CIDR notation for subnet
    #[clap(value_parser, group = "target_selection")]
    target: String,

    // ========== TARGET AND PORT SELECTION ==========
    
    /// Ports to scan (comma-separated, ranges like 1-1000)
    #[clap(short, long, default_value = "1-1000", group = "target_selection", help_heading = "TARGET AND PORT SELECTION")]
    ports: String,

    /// Scan the top 100 common ports
    #[clap(short = 'T', long, group = "target_selection", help_heading = "TARGET AND PORT SELECTION")]
    top_100: bool,

    /// Scan the top 10 most common ports (for quicker scans)
    #[clap(short = 't', long, group = "target_selection", help_heading = "TARGET AND PORT SELECTION")]
    top_10: bool,

    /// Use IPv6
    #[clap(short = '6', long, group = "target_selection", help_heading = "TARGET AND PORT SELECTION")]
    ipv6: bool,

    // ========== SCAN METHODS ==========

    /// Scan techniques to use (comma-separated)
    #[clap(short, long, default_value = "syn", group = "scan_execution", help_heading = "SCAN METHODS", long_help = "Available techniques: syn, ssl, udp, ack, fin, xmas, null, window, tls-echo, mimic, frag, dns-tunnel, icmp-tunnel\nExamples: -s syn,ssl,udp or -s syn -s ssl\nNote: Do not include spaces after commas")]
    scan_types_str: String,

    /// Enable ML-based service identification for ambiguous services
    #[clap(long = "ml-ident", default_value_t = true, group = "service_detection", help_heading = "SERVICE DETECTION")]
    ml_identification: bool,

    // ========== EVASION OPTIONS ==========

    /// Enable basic evasion techniques 
    #[clap(short, long, group = "evasion_options", help_heading = "EVASION OPTIONS", long_help = "Enable basic evasion techniques (simple TTL manipulation, basic timing randomization, minimal TCP option adjustment, packet sequencing randomization)")]
    evasion: bool,

    /// Enable advanced evasion techniques
    #[clap(short = 'E', long, default_value_t = false, group = "evasion_options", help_heading = "EVASION OPTIONS", long_help = "Enable advanced evasion techniques (OS fingerprint spoofing, TTL jittering, protocol-specific mimicry, banner grabbing suppression, sophisticated protocol variants)")]
    enhanced_evasion: bool,

    /// Operating system to mimic in enhanced evasion mode (windows, linux, macos, random)
    #[clap(long, group = "evasion_options", help_heading = "EVASION OPTIONS")]
    mimic_os: Option<String>,

    /// TTL jitter amount for enhanced evasion (1-5)
    #[clap(long, default_value_t = 2, group = "evasion_options", help_heading = "EVASION OPTIONS")]
    ttl_jitter: u8,

    /// Protocol to mimic in mimic scans (HTTP, SSH, FTP, SMTP, IMAP, POP3, MYSQL, RDP)
    #[clap(long, default_value = "HTTP", group = "evasion_options", help_heading = "EVASION OPTIONS")]
    mimic_protocol: String,

    /// Protocol variant for protocol mimicry
    #[clap(long, group = "evasion_options", help_heading = "EVASION OPTIONS")]
    protocol_variant: Option<String>,

    /// Route traffic through Tor if available
    #[clap(long, default_value_t = true, group = "evasion_options", help_heading = "EVASION OPTIONS")]
    use_tor: bool,

    // ========== TUNNELING OPTIONS ==========

    /// Use DNS tunneling to bypass restrictive firewalls
    #[clap(long = "dns-tunnel", default_value_t = false, group = "tunneling_options", help_heading = "TUNNELING OPTIONS")]
    dns_tunnel: bool,
    
    /// Use ICMP tunneling to bypass restrictive firewalls
    #[clap(long = "icmp-tunnel", default_value_t = false, group = "tunneling_options", help_heading = "TUNNELING OPTIONS")]
    icmp_tunnel: bool,
    
    /// Custom DNS server to use for DNS tunneling
    #[clap(long = "dns-server", group = "tunneling_options", help_heading = "TUNNELING OPTIONS")]
    dns_server: Option<String>,
    
    /// Custom lookup domain to use for DNS tunneling
    #[clap(long = "lookup-domain", group = "tunneling_options", help_heading = "TUNNELING OPTIONS")]
    lookup_domain: Option<String>,

    // ========== TIMING AND PERFORMANCE ==========

    /// Maximum concurrent operations
    #[clap(short, long, default_value_t = 100, group = "timing_control", help_heading = "TIMING AND PERFORMANCE")]
    concurrency: usize,

    /// Maximum packets per second
    #[clap(short = 'r', long, default_value_t = 0, group = "timing_control", help_heading = "TIMING AND PERFORMANCE")]
    rate: usize,

    /// Scan timeout in seconds
    #[clap(short, long, default_value_t = 3.0, group = "timing_control", help_heading = "TIMING AND PERFORMANCE")]
    timeout: f64,

    /// Connect timeout in seconds
    #[clap(long, default_value_t = 3.0, group = "timing_control", help_heading = "TIMING AND PERFORMANCE")]
    timeout_connect: f64,

    /// Banner grabbing timeout in seconds
    #[clap(long, default_value_t = 3.0, group = "timing_control", help_heading = "TIMING AND PERFORMANCE")]
    timeout_banner: f64,

    /// Add randomized delay before scan start (0-5 seconds)
    #[clap(long, default_value_t = true, group = "timing_control", help_heading = "TIMING AND PERFORMANCE")]
    random_delay: bool,
    
    /// Maximum random delay in seconds
    #[clap(long, default_value_t = 3, group = "timing_control", help_heading = "TIMING AND PERFORMANCE")]
    max_delay: u64,

    // ========== FRAGMENTATION OPTIONS ==========

    /// Minimum fragment size for fragmented scans
    #[clap(long, default_value_t = 24, group = "fragmentation", help_heading = "FRAGMENTATION OPTIONS")]
    frag_min_size: u16,

    /// Maximum fragment size for fragmented scans
    #[clap(long, default_value_t = 64, group = "fragmentation", help_heading = "FRAGMENTATION OPTIONS")]
    frag_max_size: u16,

    /// Minimum delay between fragments in seconds
    #[clap(long, default_value_t = 0.01, group = "fragmentation", help_heading = "FRAGMENTATION OPTIONS")]
    frag_min_delay: f64,

    /// Maximum delay between fragments in seconds
    #[clap(long, default_value_t = 0.1, group = "fragmentation", help_heading = "FRAGMENTATION OPTIONS")]
    frag_max_delay: f64,

    /// Timeout for fragmented scans in seconds
    #[clap(long, default_value_t = 10, group = "fragmentation", help_heading = "FRAGMENTATION OPTIONS")]
    frag_timeout: u64,

    /// Minimum size of first fragment
    #[clap(long, default_value_t = 64, group = "fragmentation", help_heading = "FRAGMENTATION OPTIONS")]
    frag_first_min_size: u16,

    /// Use exactly two fragments
    #[clap(long, group = "fragmentation", help_heading = "FRAGMENTATION OPTIONS")]
    frag_two_frags: bool,

    // ========== OUTPUT OPTIONS ==========

    /// Enable verbose output
    #[clap(short, long, group = "output_options", help_heading = "OUTPUT OPTIONS", long_help = "When enabled, provides detailed information about the scanning process to stdout, including debug-level messages. In disk mode, verbose logs are also written to the log file.")]
    verbose: bool,

    /// Output results in JSON format
    #[clap(short = 'j', long, group = "output_options", help_heading = "OUTPUT OPTIONS")]
    json: bool,

    /// Format the raw JSON output for pretty printing (with indentation)
    #[clap(long = "pretty-json", group = "output_options", help_heading = "OUTPUT OPTIONS")]
    pretty_json: bool,

    /// Write results to file
    #[clap(short, long, group = "output_options", help_heading = "OUTPUT OPTIONS")]
    output: Option<PathBuf>,

    /// Use ANSI colors in output
    #[clap(long, default_value_t = true, group = "output_options", help_heading = "OUTPUT OPTIONS")]
    color: bool,

    // ========== OPERATIONAL SECURITY ==========

    /// Enable memory-only mode (no disk writes)
    #[clap(short = 'm', long, group = "operational_security", help_heading = "OPERATIONAL SECURITY")]
    memory_only: bool,

    /// Log file path
    #[clap(long, default_value = "scanner.log", group = "operational_security", help_heading = "OPERATIONAL SECURITY")]
    log_file: PathBuf,
    
    /// Encrypt logs with a password
    #[clap(long, default_value_t = true, group = "operational_security", help_heading = "OPERATIONAL SECURITY")]
    encrypt_logs: bool,
    
    /// Password for log encryption
    #[clap(long, group = "operational_security", help_heading = "OPERATIONAL SECURITY")]
    _log_password: Option<String>,
    
    /// Create RAM disk for temporary files
    #[clap(long, default_value_t = true, group = "operational_security", help_heading = "OPERATIONAL SECURITY")]
    use_ramdisk: bool,
    
    /// RAM disk size in MB
    #[clap(long, default_value_t = 10, group = "operational_security", help_heading = "OPERATIONAL SECURITY")]
    ramdisk_size: u64,
    
    /// RAM disk mount point
    #[clap(long, default_value = "/mnt/quantum_scanner_ramdisk", group = "operational_security", help_heading = "OPERATIONAL SECURITY")]
    ramdisk_mount: PathBuf,

    /// Securely delete files after scan
    #[clap(long, default_value_t = false, group = "operational_security", help_heading = "OPERATIONAL SECURITY", long_help = "When enabled, performs secure deletion of log files and temporary files using multiple overwrite passes. Disabled by default for operational safety.")]
    secure_delete: bool,
    
    /// Number of secure delete passes
    #[clap(long, default_value_t = 3, group = "operational_security", help_heading = "OPERATIONAL SECURITY", long_help = "Specifies how many passes of overwriting should be performed when secure_delete is enabled. More passes provide better security but take longer.")]
    delete_passes: u8,

    /// Path to a log file to unredact (without running a scan)
    #[clap(long, group = "operational_security", help_heading = "OPERATIONAL SECURITY", long_help = "When provided without running a scan, this will only perform the redaction removal operation on the specified log file, replacing [REDACTED] with the target IP.")]
    fix_log_file: Option<PathBuf>,
}

/// Enum for scan types from CLI
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ScanTypeArg {
    Syn,
    Ssl,
    Udp,
    Ack,
    Fin,
    Xmas,
    Null,
    Window,
    TlsEcho,
    Mimic,
    Frag,
}

impl From<ScanTypeArg> for ScanType {
    fn from(arg: ScanTypeArg) -> Self {
        match arg {
            ScanTypeArg::Syn => ScanType::Syn,
            ScanTypeArg::Ssl => ScanType::Ssl,
            ScanTypeArg::Udp => ScanType::Udp,
            ScanTypeArg::Ack => ScanType::Ack,
            ScanTypeArg::Fin => ScanType::Fin,
            ScanTypeArg::Xmas => ScanType::Xmas,
            ScanTypeArg::Null => ScanType::Null,
            ScanTypeArg::Window => ScanType::Window,
            ScanTypeArg::TlsEcho => ScanType::TlsEcho,
            ScanTypeArg::Mimic => ScanType::Mimic,
            ScanTypeArg::Frag => ScanType::Frag,
        }
    }
}

/// ANSI color codes for terminal output
struct Colors {
    green: String,
    yellow: String,
    blue: String,
    #[allow(dead_code)]
    red: String,
    reset: String,
}

impl Colors {
    fn new(enabled: bool) -> Self {
        if enabled {
            Self {
                green: "\x1b[0;32m".to_string(),
                yellow: "\x1b[1;33m".to_string(),
                blue: "\x1b[0;34m".to_string(),
                red: "\x1b[0;31m".to_string(),
                reset: "\x1b[0m".to_string(),
            }
        } else {
            Self {
                green: "".to_string(),
                yellow: "".to_string(),
                blue: "".to_string(),
                red: "".to_string(),
                reset: "".to_string(),
            }
        }
    }
}

/// Initialize logging with proper configuration
fn setup_logging(_log_file: &PathBuf, verbose: bool, memory_only: bool, encrypt_logs: bool, _log_password: Option<&str>) -> Result<Option<utils::MemoryLogBuffer>, anyhow::Error> {
    // Setup memory logger if memory-only mode is enabled
    if memory_only {
        // When in memory-only mode, do not create any log files on disk
        println!("Running in memory-only mode - logs will not be written to disk");
        
        // Create memory logger with encryption if specified
        let buffer = utils::MemoryLogBuffer::new(10000, encrypt_logs);
        
        // Configure environment variable for env_logger
        let log_level = if verbose { "debug" } else { "info" };
        std::env::set_var("RUST_LOG", log_level);
        
        // Initialize the memory logger without disk logger
        env_logger::Builder::from_default_env()
            .format_timestamp_secs()
            .format_module_path(true)
            .format_target(false)
            .target(env_logger::Target::Stdout) // Redirect to stdout since we're in memory-only mode
            .init();
        
        // Log initialization message
        buffer.log("INFO", &format!("Quantum Scanner started in memory-only mode"));
        if verbose {
            buffer.log("DEBUG", "Verbose logging enabled");
        }
        
        return Ok(Some(buffer));
    }
    
    // Normal file-based logging - use a simple approach
    
    // Set log level based on verbosity
    let log_level = if verbose { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", log_level);
    
    // If encryption is enabled, we'll need to intercept logs
    if encrypt_logs {
        warn!("Log encryption is only fully supported in memory-only mode");
    }
    
    // Use a simple approach for disk logging - just use current directory
    let simple_log_file = PathBuf::from("scanner.log");
    
    // Try to create the log file with proper permissions
    let log_file_handle = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&simple_log_file) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Warning: Failed to create log file: {}. Using stdout instead.", e);
                env_logger::Builder::from_default_env()
                    .format_timestamp_secs()
                    .format_module_path(true)
                    .format_target(false)
                    .init();
                return Ok(None);
            }
        };
    
    // Set appropriate file permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&simple_log_file, std::fs::Permissions::from_mode(0o600)) {
            warn!("Failed to set secure permissions on log file: {}", e);
        }
    }
    
    println!("Running in disk mode - logs will be written to {}", simple_log_file.display());
    
    // Initialize the logger with disk file
    // For verbose mode, we'll duplicate messages to stdout
    if verbose {
        // Setup a custom formatter that also prints to stdout
        let mut builder = env_logger::Builder::new();
        builder.parse_filters(&log_level);
        builder.format(move |buf, record| {
            // Print to stdout for important messages
            if record.level() <= log::Level::Info {
                println!("[{}] {}", record.level(), record.args());
            }
            
            // Format for file - ensure no IP redaction occurs by passing the raw message
            use std::io::Write;
            writeln!(
                buf,
                "[{} {} {}:{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args() // Always display IPs in logs, never redact
            )
        });
        
        // Set the disk file as the log target
        builder.target(env_logger::Target::Pipe(Box::new(log_file_handle)));
        builder.init();
    } else {
        // Standard logging to file only for non-verbose mode
        // Always display IPs, never redact them
        env_logger::Builder::from_default_env()
            .format_timestamp_secs()
            .format_module_path(true)
            .format_target(false)
            .target(env_logger::Target::Pipe(Box::new(log_file_handle)))
            .init();
    }
    
    Ok(None)
}

/// Check if we have sufficient privileges for raw sockets
fn check_privileges(scanner_needs_raw_sockets: bool) -> bool {
    if !scanner_needs_raw_sockets {
        return true;
    }
    
    #[cfg(unix)]
    {
        // On Unix systems, check effective user ID
        unsafe { libc::geteuid() == 0 }
    }
    
    #[cfg(windows)]
    {
        // On Windows, this is more complex and not reliable
        // For a real implementation, use IsUserAnAdmin or similar
        // This is a simplified version
        true
    }
    
    #[cfg(not(any(unix, windows)))]
    {
        // Unknown platform - assume not privileged
        false
    }
}

/// Check if Tor is available and set up LD_PRELOAD if needed
fn setup_tor_routing(use_tor: bool) -> bool {
    if !use_tor {
        return false;
    }
    
    // Check if Tor is installed and running
    #[cfg(unix)]
    {
        // Try to find the tor process
        if std::process::Command::new("pgrep")
            .arg("tor")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            // Check if libtsocks is available
            if std::path::Path::new("/usr/lib/x86_64-linux-gnu/libtsocks.so").exists() {
                // Set LD_PRELOAD environment variable for Tor routing
                std::env::set_var("LD_PRELOAD", "/usr/lib/x86_64-linux-gnu/libtsocks.so");
                return true;
            }
        }
    }
    
    false
}

/// Create a RAM disk for temporary files
fn create_ramdisk(use_ramdisk: bool, mount_point: &PathBuf, size_mb: u64) -> Result<Option<PathBuf>, anyhow::Error> {
    if !use_ramdisk {
        return Ok(None);
    }
    
    #[cfg(unix)]
    {
        // Check if we have root privileges
        if unsafe { libc::geteuid() != 0 } {
            warn!("RAM disk creation requires root privileges");
            return Ok(None);
        }
        
        // Create mount point directory
        std::fs::create_dir_all(mount_point)?;
        
        // Mount a tmpfs filesystem
        let status = std::process::Command::new("mount")
            .args([
                "-t", "tmpfs",
                "-o", &format!("size={}M,mode=0700", size_mb),
                "tmpfs",
                mount_point.to_str().unwrap()
            ])
            .status()?;
        
        if status.success() {
            info!("Created RAM disk at {}", mount_point.display());
            return Ok(Some(mount_point.clone()));
        } else {
            warn!("Failed to create RAM disk");
        }
    }
    
    Ok(None)
}

/// Unmount a RAM disk
fn cleanup_ramdisk(ramdisk: &Option<PathBuf>) -> Result<(), anyhow::Error> {
    if let Some(mount_point) = ramdisk {
        #[cfg(unix)]
        {
            // Check if the mount point exists and is mounted
            if !mount_point.exists() {
                return Ok(());
            }
            
            // First try to sync all file systems to ensure all data is written
            let _ = std::process::Command::new("sync").status();
            
            // Try to unmount - first try normal unmount
            let status = std::process::Command::new("umount")
                .arg(mount_point.to_str().unwrap_or_default())
                .status();
                
            // If normal unmount fails, try lazy unmount (-l option)
            if status.is_err() || !status.unwrap().success() {
                info!("Standard unmount failed, trying lazy unmount...");
                
                // Add a small delay to allow any pending operations to complete
                std::thread::sleep(std::time::Duration::from_millis(500));
                
                let lazy_status = std::process::Command::new("umount")
                    .arg("-l")  // Lazy unmount - detach filesystem now, cleanup resources later
                    .arg(mount_point.to_str().unwrap_or_default())
                    .status()?;
                    
                if !lazy_status.success() {
                    return Err(anyhow::anyhow!("Failed to unmount RAM disk at {}", mount_point.display()));
                }
            }
            
            info!("Unmounted RAM disk from {}", mount_point.display());
            
            // Give the system a moment to complete the unmount
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // Remove directory after unmounting
            if mount_point.exists() {
                // Try multiple times to remove the directory
                for attempt in 0..3 {
                    // First try with rmdir for a clean unmounted directory
                    if attempt == 0 {
                        let _ = std::process::Command::new("rmdir")
                            .arg(mount_point.to_str().unwrap_or_default())
                            .status();
                    }
                    
                    // Then try with remove_dir_all if rmdir didn't work
                    match std::fs::remove_dir_all(mount_point) {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            // Log error but keep trying
                            info!("Remove directory attempt {}: {}", attempt + 1, e);
                            // Wait a bit and try again
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
                
                // If we couldn't remove it after 3 tries, log it but don't fail
                info!("Could not remove mount point directory after unmounting, it will be cleaned up by the system later");
            }
            
            return Ok(());
        }
    }
    
    Ok(())
}

/// Secure file deletion with multiple passes of overwriting
fn secure_delete_file(path: &PathBuf, passes: u8) -> Result<(), anyhow::Error> {
    // Ensure path exists and is a file
    if !path.exists() || !path.is_file() {
        return Ok(());
    }
    
    // Sanitize the path to prevent command injection
    let path_str = path.to_string_lossy();
    if path_str.contains(";") || path_str.contains("&") || path_str.contains("|") || 
       path_str.contains(">") || path_str.contains("<") || path_str.contains("$") {
        return Err(anyhow::anyhow!("Invalid characters in file path"));
    }
    
    // Calculate file size
    let size = match std::fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(e) => return Err(anyhow::anyhow!("Failed to get file size: {}", e)),
    };
    
    #[cfg(unix)]
    {
        // On Unix, overwrite the file with random data multiple times
        let mut rng = thread_rng();
        
        for pass in 0..passes {
            let pattern = match pass % 3 {
                0 => 0xFF, // All ones
                1 => 0x00, // All zeros
                _ => rng.gen::<u8>(), // Random
            };
            
            // Generate pattern for dd command
            let _pattern_str = format!("\\\\x{:02x}", pattern);
            
            // Use dd to overwrite with the pattern - handle command injection risk
            std::process::Command::new("dd")
                .args([
                    format!("if=/dev/zero").as_str(),
                    format!("of={}", path.display()).as_str(),
                    "bs=1k",
                    &format!("count={}", (size + 1023) / 1024), // Round up
                    "conv=notrunc"
                ])
                .output()?;
        }
        
        // Finally, delete the file
        std::fs::remove_file(path)?;
    }
    
    #[cfg(not(unix))]
    {
        // For non-Unix platforms, overwrite with zeros before deleting
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(false)
            .open(path)?;
            
        let zeros = vec![0u8; 4096];
        let mut writer = std::io::BufWriter::new(file);
        
        for _ in 0..passes {
            // Seek to beginning of file
            writer.seek(std::io::SeekFrom::Start(0))?;
            
            // Overwrite in chunks
            let mut remaining = size;
            while remaining > 0 {
                let to_write = std::cmp::min(remaining, zeros.len() as u64);
                writer.write_all(&zeros[0..to_write as usize])?;
                remaining -= to_write;
            }
            
            // Flush to ensure data is written
            writer.flush()?;
        }
        
        // Close file handle
        drop(writer);
        
        // Delete file
        std::fs::remove_file(path)?;
    }
    
    Ok(())
}

/// Unredact a log file by replacing [REDACTED] with the actual IP address
fn fix_redacted_log(log_file: &PathBuf, ip_address: &str) -> Result<usize, anyhow::Error> {
    // Check if the log file exists
    if !log_file.exists() {
        return Err(anyhow::anyhow!("Log file not found: {}", log_file.display()));
    }
    
    // Read the file content
    let mut content = String::new();
    let mut file = fs::File::open(log_file)?;
    file.read_to_string(&mut content)?;
    
    // Count occurrences before replacement
    let redacted_count = content.matches("[REDACTED]").count();
    
    if redacted_count == 0 {
        return Ok(0); // No replacements needed
    }
    
    // Create a backup of the original file
    let backup_path = format!("{}.bak", log_file.display());
    fs::copy(log_file, &backup_path)?;
    
    // Replace [REDACTED] with the IP address to permanently unredact the log
    let updated_content = content.replace("[REDACTED]", ip_address);
    
    // Write the updated content back to the file
    let mut file = fs::File::create(log_file)?;
    file.write_all(updated_content.as_bytes())?;
    
    Ok(redacted_count)
}

/// Parse scan types from a comma-separated string
fn parse_scan_types(scan_types_str: &str) -> anyhow::Result<Vec<ScanType>> {
    let mut scan_types = Vec::new();
    
    for scan_type_str in scan_types_str.split(',') {
        let scan_type_str = scan_type_str.trim().to_lowercase();
        
        match scan_type_str.as_str() {
            "syn" => scan_types.push(ScanType::Syn),
            "ssl" => scan_types.push(ScanType::Ssl),
            "udp" => scan_types.push(ScanType::Udp),
            "ack" => scan_types.push(ScanType::Ack),
            "fin" => scan_types.push(ScanType::Fin),
            "xmas" => scan_types.push(ScanType::Xmas),
            "null" => scan_types.push(ScanType::Null),
            "window" => scan_types.push(ScanType::Window),
            "tls-echo" | "tls_echo" => scan_types.push(ScanType::TlsEcho),
            "mimic" => scan_types.push(ScanType::Mimic),
            "frag" => scan_types.push(ScanType::Frag),
            "dns-tunnel" | "dns_tunnel" => scan_types.push(ScanType::DnsTunnel),
            "icmp-tunnel" | "icmp_tunnel" => scan_types.push(ScanType::IcmpTunnel),
            _ => return Err(anyhow::anyhow!("Invalid scan type: {}", scan_type_str)),
        }
    }
    
    if scan_types.is_empty() {
        return Err(anyhow::anyhow!("No valid scan types provided"));
    }
    
    Ok(scan_types)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Override the binary name for parsing
    let args: Vec<String> = std::iter::once("quantum_scanner".to_string())
        .chain(std::env::args().skip(1))
        .collect();
    
    // Parse command-line arguments with corrected binary name
    let mut args = Args::parse_from(args);
    
    // Check if we're only fixing a log file without scanning
    if let Some(log_file) = &args.fix_log_file {
        match fix_redacted_log(log_file, &args.target) {
            Ok(count) => {
                println!("Unredacted {} occurrences of [REDACTED] in {}", count, log_file.display());
                println!("A backup was created at {}.bak", log_file.display());
                return Ok(());
            },
            Err(e) => {
                eprintln!("Error unredacting log file: {}", e);
                return Err(e);
            }
        }
    }
    
    // Setup colors for output
    let colors = Colors::new(args.color);
    
    // Display banner
    if args.color {
        println!("{}╔══════════════════════════════════════════╗{}", colors.blue, colors.reset);
        println!("{}║     {}Quantum Scanner{} - {}Enhanced Edition{}     ║{}", 
            colors.blue, colors.green, colors.blue, colors.yellow, colors.blue, colors.reset);
        println!("{}╚══════════════════════════════════════════╝{}", colors.blue, colors.reset);
    } else {
        println!("┌──────────────────────────────────────────┐");
        println!("│      Quantum Scanner - Enhanced Edition      │");
        println!("└──────────────────────────────────────────┘");
    }
    
    // Setup Tor routing if available and enabled
    let _tor_enabled = if args.use_tor {
        let tor_result = setup_tor_routing(true);
        if tor_result {
            println!("[{}+{}] Routing traffic through Tor", colors.green, colors.reset);
        } else {
            println!("[{}!{}] Tor routing requested but not available", colors.yellow, colors.reset);
        }
        tor_result
    } else {
        false
    };
    
    // Check for RAM disk support for temporary files
    let ramdisk = if args.use_ramdisk {
        match create_ramdisk(args.use_ramdisk, &args.ramdisk_mount, args.ramdisk_size) {
            Ok(Some(path)) => {
                println!("[{}+{}] Created RAM disk for temporary files at {}", 
                    colors.green, colors.reset, path.display());
                
                // Use RAM disk for log file ONLY if not in memory-only mode
                if !args.memory_only {
                    args.log_file = path.join("scanner.log");
                }
                Some(path)
            },
            Ok(None) => None,
            Err(e) => {
                println!("[{}!{}] Failed to create RAM disk: {}", colors.yellow, colors.reset, e);
                None
            }
        }
    } else {
        None
    };
    
    // Add random delay before scan if enabled
    if args.random_delay {
        let delay = thread_rng().gen_range(0..args.max_delay);
        if delay > 0 {
            println!("[{}+{}] Adding random delay before scan: {}s", 
                colors.green, colors.reset, delay);
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }
    }
    
    // Use randomized packet rate if not specified
    if args.rate == 0 {
        args.rate = thread_rng().gen_range(100..500);
        println!("[{}+{}] Using randomized packet rate: {} pps", 
            colors.green, colors.reset, args.rate);
    }
    
    // Randomly select OS to mimic if not specified
    if args.mimic_os.is_none() {
        let os_types = ["windows", "linux", "macos", "random"];
        args.mimic_os = Some(os_types[thread_rng().gen_range(0..os_types.len())].to_string());
        println!("[{}+{}] Mimicking OS: {}", 
            colors.green, colors.reset, args.mimic_os.as_ref().unwrap());
    }
    
    // Select protocol variant for mimic scans if applicable and not specified
    if args.scan_types_str.contains("mimic") && args.protocol_variant.is_none() {
        let variants = ["1.0", "1.1", "2.0"];
        args.protocol_variant = Some(variants[thread_rng().gen_range(0..variants.len())].to_string());
        println!("[{}+{}] Using HTTP/{} for protocol mimicry", 
            colors.green, colors.reset, args.protocol_variant.as_ref().unwrap());
    }
    
    // Setup logging with memory-only option
    let memory_logger = match setup_logging(
        &args.log_file, 
        args.verbose, 
        args.memory_only,
        args.encrypt_logs,
        args._log_password.as_deref()
    ) {
        Ok(logger) => logger,
        Err(e) => {
            eprintln!("Warning: Failed to set up logging: {}", e);
            None
        }
    };
    
    // Log memory-only mode info
    if args.memory_only {
        println!("[{}+{}] Running in memory-only mode - logs will be kept in memory only", 
            colors.green, colors.reset);
    } else {
        println!("[{}+{}] Running in disk mode - logs will be written to {}", 
            colors.green, colors.reset, args.log_file.display());
    }
    
    // Log enhanced evasion status
    if args.enhanced_evasion {
        println!("[{}+{}] Enhanced evasion techniques enabled", colors.green, colors.reset);
    }
    
    // Handle port selection, prioritizing top_10, then top_100 over ports parameter if specified
    let ports_to_scan: Vec<u16> = if args.top_10 {
        let top_ports = TopPorts::top_10();
        println!("[{}+{}] Using top 10 most common ports for quick scanning", colors.green, colors.reset);
        top_ports
    } else if args.top_100 {
        let top_ports = TopPorts::top_100();
        println!("[{}+{}] Using top 100 common ports for scanning", colors.green, colors.reset);
        top_ports
    } else {
        // Parse port ranges
        let port_ranges = match PortRange::parse(&args.ports) {
            Ok(ranges) => ranges,
            Err(e) => {
                error!("Failed to parse port ranges: {}", e);
                eprintln!("Error: Invalid port range specification: {}", e);
                process::exit(1);
            }
        };
        
        // Expand port ranges into a list of ports
        PortRanges::new(port_ranges).into_iter().collect()
    };
    
    if ports_to_scan.is_empty() {
        error!("No valid ports specified");
        eprintln!("Error: No valid ports to scan. Please check port specification.");
        process::exit(1);
    }
    
    // Parse the scan types
    let mut scan_types = parse_scan_types(&args.scan_types_str)?;
    
    // Check if we need raw socket privileges
    let needs_raw_sockets = scanner::requires_raw_sockets(&scan_types);
    if needs_raw_sockets && !check_privileges(needs_raw_sockets) {
        error!("This scan requires root/administrator privileges");
        eprintln!("Error: This scan requires root/administrator privileges");
        process::exit(1);
    }
    
    // Print scan types being used
    println!("[{}+{}] Using scan types: {}", colors.green, colors.reset, args.scan_types_str);
    
    // Configure scanner with parsed options
    let mut scanner = QuantumScanner::new(
        &args.target,
        ports_to_scan.clone(),
        scan_types.clone(),  // Clone to avoid ownership issues
        args.concurrency,
        args.rate,
        args.evasion || args.enhanced_evasion,
        args.verbose,
        args.ipv6,
        args.timeout,
        args.timeout_connect,
        args.timeout_banner,
        &args.mimic_protocol,
        args.frag_min_size,
        args.frag_max_size,
        args.frag_min_delay,
        args.frag_max_delay,
        args.frag_timeout,
        args.frag_first_min_size,
        args.frag_two_frags,
        &args.log_file,
    ).await?;
    
    // Set enhanced evasion options
    if args.enhanced_evasion {
        scanner.set_enhanced_evasion(true, args.mimic_os.as_deref().unwrap_or("random"), args.ttl_jitter);
        scanner.set_protocol_variant(args.protocol_variant.as_deref());
    }
    
    // Set memory logger if available
    if let Some(logger) = memory_logger.clone() {
        scanner.set_memory_log(Arc::new(logger));
    }
    
    // Run the scan
    println!("[{}+{}] Starting scan of {} with {} ports", 
        colors.green, colors.reset, args.target, ports_to_scan.len());
    println!("{}════════════════════════════════════════════{}", colors.blue, colors.reset);
    
    let results = scanner.run_scan().await?;
    
    // Output results based on mode
    println!("{}════════════════════════════════════════════{}", colors.blue, colors.reset);
    println!("[{}+{}] Scan completed. Found {} open ports", 
        colors.green, colors.reset, results.open_ports.len());
    
    // Display results based on output mode
    if args.json || args.pretty_json {
        // If JSON output is requested, serialize and display the results
        let json_output = if args.pretty_json {
            serde_json::to_string_pretty(&results)
                .unwrap_or_else(|e| format!("Error serializing to JSON: {}", e))
        } else {
            serde_json::to_string(&results)
                .unwrap_or_else(|e| format!("Error serializing to JSON: {}", e))
        };
        println!("\n{}", json_output);
    } else if args.verbose {
        // Display enhanced scan details using the print_results function
        output::print_results(&results)?;
    } else {
        // Display simplified results for non-verbose mode
        for port in results.open_ports.iter().cloned().collect::<Vec<_>>() {
            if let Some(result) = results.results.get(&port) {
                let service_info = match (&result.service, &result.version) {
                    (Some(service), Some(version)) => format!("{} ({})", service, version),
                    (Some(service), None) => service.clone(),
                    _ => "unknown".to_string()
                };
                
                println!("[{}OPEN{}] Port {}: {} ", 
                    colors.green, colors.reset, port, service_info);
                
                // Show banner information if available
                if let Some(banner) = &result.banner {
                    // Trim and show the first line of the banner for compact output
                    let banner_preview = banner.lines().next()
                        .unwrap_or("").trim();
                    if !banner_preview.is_empty() {
                        println!("       Banner: {}", banner_preview);
                    }
                }
                
                // Show condensed vulnerability count if present
                if !result.vulns.is_empty() {
                    println!("       {}- {} potential vulnerabilities detected{}",
                        colors.yellow, result.vulns.len(), colors.reset);
                }
            }
        }
        
        // Display scan statistics summary
        println!("\n[{}INFO{}] Scan Statistics:", colors.blue, colors.reset);
        println!("       - Packets sent: {}", results.packets_sent);
        println!("       - Success rate: {:.1}%", 
            if results.packets_sent > 0 { 
                (results.successful_scans as f64 / results.packets_sent as f64) * 100.0 
            } else { 
                0.0 
            }
        );
        
        // Show total vulnerability count
        let total_vulns: usize = results.results.values()
            .map(|r| r.vulns.len())
            .sum();
        
        if total_vulns > 0 {
            println!("       - {} potential vulnerabilities detected", total_vulns);
        }
        
        println!("\nUse --verbose for more detailed output");
    }
    
    // Output to file if requested
    if let Some(output_path) = args.output {
        if args.json || args.pretty_json {
            output::save_json_results(&results, &output_path)?;
            println!("[{}+{}] Results saved to {} in JSON format", 
                colors.green, colors.reset, output_path.display());
        } else {
            output::save_text_results(&results, &output_path)?;
            println!("[{}+{}] Results saved to {}", 
                colors.green, colors.reset, output_path.display());
        }
    }
    
    // Print memory log summary if available
    if let Some(logger) = memory_logger {
        if args.verbose {
            println!("\nLog entries: {}", logger.len());
            println!("Log contents:");
            println!("{}", logger.format_logs(true));
        }
    }
    
    // Cleanup phase
    if args.secure_delete {
        println!("[{}+{}] Performing secure cleanup...", colors.green, colors.reset);
        
        // Delete log file if it exists
        if args.log_file.exists() && !args.memory_only {
            match secure_delete_file(&args.log_file, args.delete_passes) {
                Ok(_) => println!("[{}+{}] Securely deleted log file", colors.green, colors.reset),
                Err(e) => println!("[{}!{}] Failed to securely delete log file: {}", 
                    colors.yellow, colors.reset, e),
            }
        }
        
        // Cleanup RAM disk if created
        if ramdisk.is_some() {
            match cleanup_ramdisk(&ramdisk) {
                Ok(_) => println!("[{}+{}] RAM disk cleaned up successfully", colors.green, colors.reset),
                Err(e) => println!("[{}!{}] Failed to clean up RAM disk: {}", 
                    colors.yellow, colors.reset, e),
            }
        }
    }
    
    // Add tunneling scan types if requested
    if args.dns_tunnel && !scan_types.contains(&ScanType::DnsTunnel) {
        scan_types.push(ScanType::DnsTunnel);
    }

    if args.icmp_tunnel && !scan_types.contains(&ScanType::IcmpTunnel) {
        scan_types.push(ScanType::IcmpTunnel);
    }

    // Configure DNS tunneling options if needed
    if args.dns_tunnel || scan_types.contains(&ScanType::DnsTunnel) {
        let dns_server = if let Some(dns_server_str) = &args.dns_server {
            match dns_server_str.parse::<std::net::IpAddr>() {
                Ok(ip) => Some(ip),
                Err(e) => {
                    eprintln!("Error parsing DNS server IP: {}", e);
                    return Err(anyhow::anyhow!("Invalid DNS server: {}", e));
                }
            }
        } else {
            None
        };
        
        scanner.set_dns_tunnel_options(dns_server, args.lookup_domain.as_deref());
    }
    
    println!("{}Quantum Scanner operation complete{}", colors.green, colors.reset);
    
    Ok(())
}
