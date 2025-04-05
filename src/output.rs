use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use console::{style, Term};
use serde_json;

use crate::models::{PortResult, PortStatus, ScanResults};
use crate::utils::sanitize_string;

/// Save scan results to a JSON file
///
/// # Arguments
/// * `results` - The scan results to save
/// * `output_path` - Path to the output file
///
/// # Returns
/// * `Result<()>` - Success or error
pub fn save_json_results(results: &ScanResults, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    let mut file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {:?}", output_path))?;
    
    file.write_all(json.as_bytes())
        .with_context(|| format!("Failed to write to output file: {:?}", output_path))?;
    
    Ok(())
}

/// Convert scan results to a simple text report format
pub fn format_text_results(results: &ScanResults) -> String {
    let mut output = String::new();
    
    // Header
    output.push_str(&format!("# Quantum Scanner Report\n"));
    output.push_str(&format!("Target: {}\n", results.target));
    output.push_str(&format!("IP: {}\n", results.target_ip));
    output.push_str(&format!("Timestamp: {}\n", Utc::now()));
    output.push_str(&format!("Scan Duration: {:.2} seconds\n", 
        results.end_time.signed_duration_since(results.start_time).num_milliseconds() as f64 / 1000.0));
    
    // Scan types used
    output.push_str("Scan types: ");
    for (i, scan_type) in results.scan_types.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        output.push_str(&scan_type.to_string());
    }
    output.push_str("\n\n");
    
    // Enhanced scan statistics
    output.push_str("## Scan Statistics\n");
    output.push_str(&format!("Packets sent: {}\n", results.packets_sent));
    output.push_str(&format!("Successful operations: {}\n", results.successful_scans));
    if results.packets_sent > 0 {
        output.push_str(&format!("Success rate: {:.1}%\n", 
                               (results.successful_scans as f64 / results.packets_sent as f64) * 100.0));
    }
    
    // OS detection summary if available
    if let Some(os_summary) = &results.os_summary {
        output.push_str(&format!("OS detection: {}\n", os_summary));
    }
    
    // Risk assessment if available
    if let Some(risk) = &results.risk_assessment {
        output.push_str(&format!("Risk assessment: {}\n", risk));
    }
    
    output.push_str("\n");
    
    // Service categories if available
    if let Some(categories) = &results.service_categories {
        output.push_str("## Service Categories\n");
        for (category, ports) in categories {
            let ports_str = ports.iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&format!("- {}: {}\n", category, ports_str));
        }
        output.push_str("\n");
    }
    
    // Open ports summary
    output.push_str(&format!("## Open Ports Summary\n"));
    if results.open_ports.is_empty() {
        output.push_str("No open ports found\n\n");
    } else {
        output.push_str(&format!("Found {} open ports:\n\n", results.open_ports.len()));
        output.push_str("PORT      STATE   SERVICE         VERSION\n");
        output.push_str("----------------------------------------------------\n");
        
        // Sort ports for consistent output
        let mut ports: Vec<u16> = results.open_ports.iter().copied().collect();
        ports.sort_unstable();
        
        for port in ports {
            if let Some(port_result) = results.results.get(&port) {
                // Find the first open state
                let state = if port_result.tcp_states.values().any(|s| *s == PortStatus::Open) {
                    "open"
                } else if port_result.udp_state == Some(PortStatus::Open) {
                    "open/udp"
                } else {
                    "open|filtered"
                };
                
                let service = port_result.service.as_deref().unwrap_or("unknown");
                let version = port_result.version.as_deref().unwrap_or("");
                
                output.push_str(&format!("{:<9} {:<7} {:<15} {}\n", 
                    port, state, service, version));
            }
        }
        output.push_str("\n");
    }
    
    // Detailed port information
    output.push_str("## Port Details\n\n");
    
    // Sort ports for consistent output
    let mut all_ports: Vec<u16> = results.results.keys().copied().collect();
    all_ports.sort_unstable();
    
    for port in all_ports {
        let port_result = &results.results[&port];
        
        // Skip ports with no interesting results
        let has_data = !port_result.tcp_states.is_empty() 
            || port_result.udp_state.is_some()
            || port_result.banner.is_some()
            || port_result.cert_info.is_some()
            || port_result.vulns.len() > 0
            || port_result.anomalies.len() > 0
            || port_result.security_posture.is_some();
            
        if !has_data {
            continue;
        }
        
        output.push_str(&format!("### Port {}\n", port));
        
        // Service information
        if let Some(service) = &port_result.service {
            output.push_str(&format!("Service: {}\n", service));
        }
        
        if let Some(version) = &port_result.version {
            output.push_str(&format!("Version: {}\n", version));
        }
        
        // States by scan type
        if !port_result.tcp_states.is_empty() {
            output.push_str("TCP States:\n");
            for (scan_type, status) in &port_result.tcp_states {
                output.push_str(&format!("  - {} scan: {}\n", scan_type, status));
            }
        }
        
        if let Some(udp_state) = &port_result.udp_state {
            output.push_str(&format!("UDP State: {}\n", udp_state));
        }
        
        // Enhanced security posture assessment
        if let Some(posture) = &port_result.security_posture {
            output.push_str("Security Assessment:\n");
            for item in posture.split(';') {
                output.push_str(&format!("  - {}\n", item.trim()));
            }
        }
        
        // Enhanced anomaly detection
        if !port_result.anomalies.is_empty() {
            output.push_str("Detected Anomalies:\n");
            for anomaly in &port_result.anomalies {
                output.push_str(&format!("  - {}\n", anomaly));
            }
        }
        
        // Timing analysis if available
        if let Some(timing) = &port_result.timing_analysis {
            output.push_str(&format!("Timing Analysis: {}\n", timing));
        }
        
        // Enhanced service details
        if let Some(details) = &port_result.service_details {
            output.push_str("Service Details:\n");
            for (key, value) in details {
                output.push_str(&format!("  - {}: {}\n", key, value));
            }
        }
        
        // Banner if available
        if let Some(banner) = &port_result.banner {
            output.push_str("Banner:\n");
            // Sanitize and format the banner
            let sanitized = sanitize_banner(banner);
            for line in sanitized.lines().take(5) {
                output.push_str(&format!("  {}\n", line));
            }
        }
        
        // Certificate info if available
        if let Some(cert) = &port_result.cert_info {
            output.push_str("SSL/TLS Certificate:\n");
            output.push_str(&format!("  Subject: {}\n", cert.subject));
            output.push_str(&format!("  Issuer: {}\n", cert.issuer));
            output.push_str(&format!("  Valid from: {}\n", cert.not_before));
            output.push_str(&format!("  Valid until: {}\n", cert.not_after));
            
            if let Some(bits) = cert.public_key_bits {
                output.push_str(&format!("  Key strength: {} bits ({})\n", 
                    bits, 
                    if bits < 2048 { "weak" } else { "strong" }));
            }
            
            if !cert.alt_names.is_empty() {
                output.push_str("  Alternative Names:\n");
                for name in cert.alt_names.iter().take(5) {
                    output.push_str(&format!("    - {}\n", name));
                }
                
                if cert.alt_names.len() > 5 {
                    output.push_str(&format!("    ... and {} more\n", cert.alt_names.len() - 5));
                }
            }
        }
        
        // Vulnerabilities if found
        if !port_result.vulns.is_empty() {
            output.push_str("Vulnerabilities:\n");
            for vuln in &port_result.vulns {
                output.push_str(&format!("  - {}\n", vuln));
            }
        }
        
        output.push_str("\n");
    }
    
    output
}

/// Save scan results as formatted text
///
/// # Arguments
/// * `results` - The scan results to save
/// * `output_path` - Path to the output file
///
/// # Returns
/// * `Result<()>` - Success or error
pub fn save_text_results(results: &ScanResults, output_path: &Path) -> Result<()> {
    let text = format_text_results(results);
    let mut file = File::create(output_path)
        .context(format!("Failed to create output file: {:?}", output_path))?;
    
    file.write_all(text.as_bytes())
        .context("Failed to write text results")?;
    
    Ok(())
}

#[allow(dead_code)]
fn format_port_text(file: &mut File, port: u16, result: &PortResult) -> Result<()> {
    // Basic port info
    let port_header = format!(
        "### Port {}\n\
         Service: {}\n",
        port,
        result.service.as_deref().unwrap_or("unknown")
    );
    
    file.write_all(port_header.as_bytes())?;
    
    // Version if available
    if let Some(version) = &result.version {
        writeln!(file, "Version: {}", version)?;
    }
    
    // Scan results for each technique
    if !result.tcp_states.is_empty() {
        file.write_all(b"\nTCP Scan Results:\n")?;
        for (scan_type, status) in &result.tcp_states {
            writeln!(file, "- {}: {}", scan_type, status)?;
        }
    }
    
    // UDP result if available
    if let Some(udp_status) = &result.udp_state {
        writeln!(file, "\nUDP: {}", udp_status)?;
    }
    
    // Firewall filtering
    if let Some(filtering) = &result.filtering {
        writeln!(file, "Filtering: {}", filtering)?;
    }
    
    // Banner if available
    if let Some(banner) = &result.banner {
        writeln!(file, "\nBanner: {}", sanitize_string(banner))?;
    }
    
    // Certificate info if available
    if let Some(cert) = &result.cert_info {
        file.write_all(b"\nSSL Certificate:\n")?;
        writeln!(file, "- Subject: {}", cert.subject)?;
        writeln!(file, "- Issuer: {}", cert.issuer)?;
        writeln!(file, "- Valid: {} to {}", 
            cert.not_before,
            cert.not_after
        )?;
        writeln!(file, "- Signature Algo: {}", cert.signature_algorithm)?;
        if let Some(bits) = cert.public_key_bits {
            writeln!(file, "- Key: {} {} bits", 
                cert.key_algorithm.as_deref().unwrap_or("Unknown"),
                bits
            )?;
        }
    }
    
    // Vulnerabilities if found
    if !result.vulns.is_empty() {
        file.write_all(b"\nPotential Vulnerabilities:\n")?;
        for vuln in &result.vulns {
            writeln!(file, "- {}", vuln)?;
        }
    }
    
    // Add a separator
    file.write_all(b"\n----------\n\n")?;
    
    Ok(())
}

#[allow(dead_code)]
pub fn print_results(results: &ScanResults) -> Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;
    
    // Print header
    println!("{}", style("Quantum Scanner Results").cyan().bold());
    println!("Target: {} ({})", style(&results.target).green(), results.target_ip);
    println!("Scan Time: {} to {}", 
        results.start_time.format("%H:%M:%S"),
        results.end_time.format("%H:%M:%S")
    );
    println!("Scan Duration: {:.2} seconds", 
        (results.end_time - results.start_time).num_milliseconds() as f64 / 1000.0
    );
    
    println!("\n{} open ports discovered", style(results.open_ports.len()).yellow().bold());
    
    // If no open ports, end here
    if results.open_ports.is_empty() {
        println!("\nNo open ports found on target.");
        return Ok(());
    }
    
    // Print open ports summary with enhanced information
    println!("\n{}", style("PORT     STATE  SERVICE  VERSION          BANNER").underlined());
    
    let mut ports: Vec<_> = results.open_ports.iter().collect();
    ports.sort_unstable();
    
    for &port in ports {
        if let Some(result) = results.results.get(&port) {
            print_port_summary(port, result);
        }
    }
    
    // Print detailed information for interesting ports
    print_detailed_results(results)?;
    
    Ok(())
}

#[allow(dead_code)]
fn print_port_summary(port: u16, result: &PortResult) {
    // Get the first "open" state
    let state = result.tcp_states.iter()
        .find(|(_, status)| **status == PortStatus::Open)
        .map(|(_, status)| status.to_string())
        .unwrap_or_else(|| "open".to_string());
    
    // Format version information
    let version = result.version.as_deref().unwrap_or("-");
    
    // Format banner preview (first line, trimmed)
    let banner_preview = match &result.banner {
        Some(banner) => {
            let first_line = banner.lines().next().unwrap_or("").trim();
            if first_line.len() > 30 {
                format!("{}...", &first_line[0..27])
            } else {
                first_line.to_string()
            }
        },
        None => "-".to_string()
    };
    
    // Format the line with banner information
    println!("{:<8} {:<6} {:<8} {:<16} {}", 
        style(port).green().bold(),
        state,
        result.service.as_deref().unwrap_or("-"),
        version,
        style(&banner_preview).dim()
    );
}

#[allow(dead_code)]
fn print_detailed_results(results: &ScanResults) -> Result<()> {
    let mut ports: Vec<_> = results.open_ports.iter().collect();
    ports.sort_unstable();
    
    println!("\n{}", style("Detailed Port Information").cyan().bold());
    println!("{}", style("=========================").cyan());
    
    for &port in ports {
        if let Some(result) = results.results.get(&port) {
            println!("\n{} - {}", 
                style(format!("Port {}", port)).yellow().bold(),
                style(result.service.as_deref().unwrap_or("unknown service")).green()
            );
            
            // Print version information
            if let Some(version) = &result.version {
                println!("  Version: {}", version);
            }
            
            // Print service fingerprinting information
            if !result.tcp_states.is_empty() {
                println!("  Scan Results:");
                for (scan_type, status) in &result.tcp_states {
                    println!("    - {} scan: {}", scan_type, status);
                }
            }
            
            if let Some(udp_state) = &result.udp_state {
                println!("  UDP: {}", udp_state);
            }
            
            // Print filtering information
            if let Some(filtering) = &result.filtering {
                println!("  Filtering: {}", filtering);
            }
            
            // Print banner with proper formatting
            if let Some(banner) = &result.banner {
                println!("  {}", style("Banner:").underlined());
                // Split the banner into lines and print each with proper indentation
                for (i, line) in banner.lines().enumerate() {
                    if i >= 10 {
                        println!("    ... (truncated - {} more lines)", banner.lines().count() - 10);
                        break;
                    }
                    println!("    {}", line);
                }
            }
            
            // Print detailed service information
            if let Some(details) = &result.service_details {
                println!("  Service Details:");
                for (key, value) in details {
                    println!("    - {}: {}", key, value);
                }
            }
            
            // Print security posture assessment
            if let Some(posture) = &result.security_posture {
                println!("  Security Assessment:");
                for item in posture.split(';') {
                    if !item.trim().is_empty() {
                        println!("    - {}", item.trim());
                    }
                }
            }
            
            // Print timing analysis
            if let Some(timing) = &result.timing_analysis {
                println!("  Timing Analysis: {}", timing);
            }
            
            // Print detected anomalies
            if !result.anomalies.is_empty() {
                println!("  Detected Anomalies:");
                for anomaly in &result.anomalies {
                    println!("    - {}", anomaly);
                }
            }
            
            // Print certificate info if available
            if let Some(cert) = &result.cert_info {
                println!("  {}", style("SSL/TLS Certificate:").underlined());
                println!("    Subject: {}", cert.subject);
                println!("    Issuer: {}", cert.issuer);
                println!("    Valid from: {}", cert.not_before);
                println!("    Valid until: {}", cert.not_after);
                println!("    Signature Algorithm: {}", cert.signature_algorithm);
                
                if let Some(bits) = cert.public_key_bits {
                    let strength = if bits < 2048 { 
                        style("weak").red() 
                    } else { 
                        style("strong").green() 
                    };
                    println!("    Key strength: {} bits ({})", bits, strength);
                }
                
                if !cert.alt_names.is_empty() {
                    println!("    Alternative Names:");
                    for (i, name) in cert.alt_names.iter().enumerate() {
                        if i >= 5 {
                            println!("      ... and {} more", cert.alt_names.len() - 5);
                            break;
                        }
                        println!("      - {}", name);
                    }
                }
            }
            
            // Print vulnerabilities if found
            if !result.vulns.is_empty() {
                println!("  {}", style("Potential Vulnerabilities:").red().bold());
                for vuln in &result.vulns {
                    println!("    - {}", style(vuln).red());
                }
            }
            
            println!("{}", style("-------------------------").dim());
        }
    }
    
    // Print OS detection info if available
    if let Some(os_summary) = &results.os_summary {
        println!("\n{}", style("OS Detection:").cyan().bold());
        println!("  {}", os_summary);
    }
    
    // Print risk assessment if available
    if let Some(risk) = &results.risk_assessment {
        println!("\n{}", style("Risk Assessment:").cyan().bold());
        println!("  {}", risk);
    }
    
    // Print service categories if available
    if let Some(categories) = &results.service_categories {
        println!("\n{}", style("Service Categories:").cyan().bold());
        for (category, ports) in categories {
            let ports_str = ports.iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("  - {}: {}", style(category).yellow(), ports_str);
        }
    }
    
    Ok(())
}

#[allow(dead_code)]
fn format_duration(seconds: f64) -> String {
    if seconds < 0.001 {
        format!("{:.2} μs", seconds * 1_000_000.0)
    } else if seconds < 1.0 {
        format!("{:.2} ms", seconds * 1_000.0)
    } else if seconds < 60.0 {
        format!("{:.2} sec", seconds)
    } else {
        let minutes = (seconds / 60.0).floor();
        let secs = seconds - (minutes * 60.0);
        format!("{}m {:.2}s", minutes as u32, secs)
    }
}

#[allow(dead_code)]
pub fn print_open_ports(results: &ScanResults) -> Result<()> {
    println!("{}", style("OPEN PORTS:").cyan().bold());
    println!("{}", style("PORT      STATE   SERVICE").cyan());
    println!("{}", style("------------------------").cyan());
    
    if results.open_ports.is_empty() {
        println!("No open ports found");
        return Ok(());
    }
    
    // Sort ports for consistent output
    let mut ports: Vec<u16> = results.open_ports.iter().copied().collect();
    ports.sort_unstable();
    
    for port in ports {
        if let Some(result) = results.results.get(&port) {
            let service = result.service.as_deref().unwrap_or("unknown");
            println!("{:<9} {:<7} {}", 
                style(port).green(), 
                style("open").green(), 
                style(service).green());
        }
    }
    
    Ok(())
}

#[allow(dead_code)]
pub fn print_port_details(results: &ScanResults, port: u16) -> Result<()> {
    if let Some(result) = results.results.get(&port) {
        println!("{} {}", style("PORT DETAILS:").cyan().bold(), style(port).cyan());
        println!("{}", style("------------------------").cyan());
        
        // Service information
        if let Some(service) = &result.service {
            println!("{}: {}", style("Service").yellow(), service);
        }
        
        if let Some(version) = &result.version {
            println!("{}: {}", style("Version").yellow(), version);
        }
        
        // States by scan type
        if !result.tcp_states.is_empty() {
            println!("{}: ", style("TCP States").yellow());
            for (scan_type, status) in &result.tcp_states {
                println!("  {} scan: {}", scan_type, status);
            }
        }
        
        if let Some(udp_state) = &result.udp_state {
            println!("{}: {}", style("UDP State").yellow(), udp_state);
        }
        
        // Banner if available
        if let Some(banner) = &result.banner {
            println!("{}:", style("Banner").yellow());
            let sanitized = sanitize_banner(banner);
            for line in sanitized.lines().take(5) {
                println!("  {}", line);
            }
        }
        
        // Certificate info if available
        if let Some(cert) = &result.cert_info {
            println!("{}:", style("SSL/TLS Certificate").yellow());
            println!("  Subject: {}", cert.subject);
            println!("  Issuer: {}", cert.issuer);
            println!("  Valid from: {}", cert.not_before);
            println!("  Valid until: {}", cert.not_after);
        }
        
        // Vulnerabilities if found
        if !result.vulns.is_empty() {
            println!("{}:", style("Vulnerabilities").yellow().bold());
            for vuln in &result.vulns {
                println!("  - {}", style(vuln).red());
            }
        }
    } else {
        println!("No information available for port {}", port);
    }
    
    Ok(())
}

/// Sanitize banner string for display
fn sanitize_banner(banner: &str) -> String {
    sanitize_string(banner)
}

#[allow(dead_code)]
pub fn export_to_csv(results: &ScanResults, writer: &mut dyn Write) -> Result<()> {
    // Write CSV header
    writeln!(writer, "port,state,service,version,banner")?;
    
    // Sort ports for consistent output
    let mut ports: Vec<u16> = results.results.keys().copied().collect();
    ports.sort_unstable();
    
    for port in ports {
        if let Some(result) = results.results.get(&port) {
            // Find the most open state
            let state = if result.tcp_states.values().any(|s| *s == PortStatus::Open) {
                "open"
            } else if result.udp_state == Some(PortStatus::Open) {
                "open/udp"
            } else if result.tcp_states.values().any(|s| *s == PortStatus::OpenFiltered) {
                "open|filtered"
            } else if !result.tcp_states.is_empty() {
                "closed"
            } else {
                "unknown"
            };
            
            let service = result.service.as_deref().unwrap_or("").replace(",", "");
            let version = result.version.as_deref().unwrap_or("").replace(",", "");
            
            // Take first line of banner and escape quotes, commas
            let banner = match &result.banner {
                Some(b) => b.lines().next().unwrap_or("").replace("\"", "\"\"").replace(",", "\\,"),
                None => String::new(),
            };
            
            writeln!(writer, "{},{},{},{},\"{}\"", port, state, service, version, banner)?;
        }
    }
    
    Ok(())
} 