use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub vram_total_mb: u64,
    pub vram_used_mb: u64,
    pub vram_free_mb: u64,
    pub compute_capability: String,
    pub pcie_bandwidth_gbps: f64,
}

pub fn detect_gpu() -> Result<GpuInfo, String> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.used,memory.free,compute_cap,pcie.link.width.current,pcie.link.gen.current",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|e| format!("nvidia-smi not found or failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("nvidia-smi failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim().lines().next()
        .ok_or("No GPU found")?;

    let parts: Vec<&str> = line.split(", ").collect();
    if parts.len() < 7 {
        return Err(format!("Unexpected nvidia-smi output: {}", line));
    }

    let name = parts[0].trim().to_string();
    let vram_total_mb = parts[1].trim().parse::<u64>().unwrap_or(0);
    let vram_used_mb = parts[2].trim().parse::<u64>().unwrap_or(0);
    let vram_free_mb = parts[3].trim().parse::<u64>().unwrap_or(0);
    let compute_capability = parts[4].trim().to_string();

    // PCIe bandwidth estimation
    let pcie_width: u32 = parts[5].trim().parse().unwrap_or(16);
    let pcie_gen: u32 = parts[6].trim().parse().unwrap_or(4);
    let per_lane_gbps = match pcie_gen {
        1 => 0.25,
        2 => 0.5,
        3 => 0.985,
        4 => 1.969,
        5 => 3.938,
        6 => 7.563,
        _ => 1.969,
    };
    let pcie_bandwidth_gbps = per_lane_gbps * pcie_width as f64;

    Ok(GpuInfo {
        name,
        vram_total_mb,
        vram_used_mb,
        vram_free_mb,
        compute_capability,
        pcie_bandwidth_gbps,
    })
}

/// Get total system RAM in MB
pub fn get_system_ram_mb() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            content.lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| {
                    l.split_whitespace().nth(1)
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(|kb| kb / 1024)
                })
        })
        .unwrap_or(0)
}
