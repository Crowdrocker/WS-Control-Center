use serde::Serialize;
use std::{fs, thread};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub os_name: String,
    pub kernel: String,
    pub architecture: String,
    pub processor: String,
    pub logical_cores: usize,
    pub memory_total_bytes: u64,
    pub memory_available_bytes: u64,
    pub uptime_seconds: u64,
}

pub fn read_status() -> Result<SystemStatus, String> {
    #[cfg(target_os = "linux")]
    {
        read_linux_status()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("System status is currently available only on Linux.".to_string())
    }
}

#[cfg(target_os = "linux")]
fn read_linux_status() -> Result<SystemStatus, String> {
    let os_release = read_system_file("/etc/os-release")?;
    let os_name = parse_os_name(&os_release).ok_or_else(|| {
        "The Linux distribution name was not found in /etc/os-release.".to_string()
    })?;
    let cpu_info = read_system_file("/proc/cpuinfo")?;
    let memory_info = read_system_file("/proc/meminfo")?;
    let uptime_info = read_system_file("/proc/uptime")?;
    let (memory_total_bytes, memory_available_bytes) = parse_memory(&memory_info)?;
    let uptime_seconds = parse_uptime(&uptime_info)?;

    Ok(SystemStatus {
        os_name,
        kernel: read_system_file("/proc/sys/kernel/osrelease")?
            .trim()
            .to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        processor: parse_processor(&cpu_info)
            .unwrap_or_else(|| "Processor name unavailable".to_string()),
        logical_cores: parse_logical_cores(&cpu_info)
            .or_else(|| thread::available_parallelism().ok().map(usize::from))
            .ok_or_else(|| "Could not determine the logical CPU count.".to_string())?,
        memory_total_bytes,
        memory_available_bytes,
        uptime_seconds,
    })
}

#[cfg(target_os = "linux")]
fn read_system_file(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("Could not read {path}: {error}"))
}

fn parse_os_name(contents: &str) -> Option<String> {
    let raw = contents
        .lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))?
        .trim();
    let value = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            raw.strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(raw);
    let value = value.replace("\\\"", "\"").replace("\\\\", "\\");
    (!value.is_empty()).then_some(value)
}

fn parse_processor(contents: &str) -> Option<String> {
    ["model name", "Hardware"].into_iter().find_map(|key| {
        contents.lines().find_map(|line| {
            let (field, value) = line.split_once(':')?;
            (field.trim() == key && !value.trim().is_empty()).then(|| value.trim().to_string())
        })
    })
}

fn parse_logical_cores(contents: &str) -> Option<usize> {
    let count = contents
        .lines()
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(field, _)| field.trim() == "processor")
        })
        .count();
    (count > 0).then_some(count)
}

fn parse_memory(contents: &str) -> Result<(u64, u64), String> {
    fn value_kib(contents: &str, key: &str) -> Option<u64> {
        let value = contents.lines().find_map(|line| {
            let (field, value) = line.split_once(':')?;
            (field.trim() == key).then(|| value.split_whitespace().next()?.parse::<u64>().ok())?
        })?;
        value.checked_mul(1024)
    }

    let total = value_kib(contents, "MemTotal")
        .ok_or_else(|| "Could not parse total memory from /proc/meminfo.".to_string())?;
    let available = value_kib(contents, "MemAvailable")
        .ok_or_else(|| "Could not parse available memory from /proc/meminfo.".to_string())?;
    if total == 0 || available > total {
        return Err("The memory values in /proc/meminfo are inconsistent.".to_string());
    }
    Ok((total, available))
}

fn parse_uptime(contents: &str) -> Result<u64, String> {
    let seconds = contents
        .split_whitespace()
        .next()
        .ok_or_else(|| "Linux did not report system uptime.".to_string())?
        .parse::<f64>()
        .map_err(|error| format!("Could not parse system uptime: {error}"))?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("Linux reported an invalid system uptime.".to_string());
    }
    Ok(seconds.floor() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_distribution_name_from_os_release() {
        assert_eq!(
            parse_os_name("NAME=Example\nPRETTY_NAME=\"Example Linux 1\"\n"),
            Some("Example Linux 1".to_string())
        );
        assert_eq!(
            parse_os_name("PRETTY_NAME='Example Linux'"),
            Some("Example Linux".to_string())
        );
        assert_eq!(parse_os_name("NAME=Example"), None);
    }

    #[test]
    fn parses_memory_values_from_proc_meminfo() {
        assert_eq!(
            parse_memory("MemTotal:       16384 kB\nMemAvailable:    8192 kB\n").unwrap(),
            (16 * 1024 * 1024, 8 * 1024 * 1024)
        );
        assert!(parse_memory("MemTotal: 10 kB\nMemAvailable: 11 kB\n").is_err());
    }

    #[test]
    fn parses_processor_and_logical_core_count() {
        let cpu_info = "processor : 0\nmodel name : Example CPU\nprocessor : 1\n";
        assert_eq!(parse_processor(cpu_info).as_deref(), Some("Example CPU"));
        assert_eq!(parse_logical_cores(cpu_info), Some(2));
    }

    #[test]
    fn parses_uptime_seconds() {
        assert_eq!(parse_uptime("86400.9 1000.0").unwrap(), 86400);
        assert!(parse_uptime("-1 0").is_err());
        assert!(parse_uptime("").is_err());
    }
}
