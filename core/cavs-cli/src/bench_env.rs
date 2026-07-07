//! Benchmark environment capture (v0.9.0): every benchmark report
//! records where and how it was produced — OS, CPU, RAM, disk type,
//! tool versions, the exact command line and the dataset seed — so a
//! result is reproducible or comparable, never anonymous.

use serde::Serialize;
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct BenchEnvironment {
    pub os: String,
    pub arch: String,
    pub cpu: String,
    pub ram_gib: Option<f64>,
    pub disk: String,
    pub cavs_version: String,
    pub tool_versions: Vec<(String, String)>,
    /// The exact invocation that produced the report.
    pub command: String,
    /// Dataset generator seed (deterministic: same seed ⇒ same bytes).
    pub seed: u64,
}

fn stdout_of(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    let text = String::from_utf8_lossy(if out.stdout.is_empty() {
        &out.stderr
    } else {
        &out.stdout
    });
    let line = text.lines().next()?.trim().to_string();
    (!line.is_empty()).then_some(line)
}

fn cpu_model() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Some(cpu) = stdout_of("sysctl", &["-n", "machdep.cpu.brand_string"]) {
            return cpu;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in info.lines() {
                if let Some(model) = line.strip_prefix("model name") {
                    return model.trim_start_matches([':', '\t', ' ']).to_string();
                }
            }
        }
    }
    "unknown".into()
}

fn ram_gib() -> Option<f64> {
    #[cfg(target_os = "macos")]
    {
        let bytes: u64 = stdout_of("sysctl", &["-n", "hw.memsize"])?.parse().ok()?;
        return Some(bytes as f64 / (1u64 << 30) as f64);
    }
    #[cfg(target_os = "linux")]
    {
        let info = std::fs::read_to_string("/proc/meminfo").ok()?;
        let kb: u64 = info
            .lines()
            .find(|l| l.starts_with("MemTotal"))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()?;
        return Some(kb as f64 / (1u64 << 20) as f64);
    }
    #[allow(unreachable_code)]
    None
}

fn os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Some(v) = stdout_of("sw_vers", &["-productVersion"]) {
            let kernel = stdout_of("uname", &["-r"]).unwrap_or_default();
            return format!("macOS {v} (Darwin {kernel})");
        }
    }
    if let Some(u) = stdout_of("uname", &["-sr"]) {
        return u;
    }
    std::env::consts::OS.into()
}

fn disk_type() -> String {
    // Best effort: on Apple Silicon Macs the internal disk is NVMe/APFS;
    // elsewhere report the filesystem of the current directory.
    #[cfg(target_os = "macos")]
    {
        "APFS (internal NVMe SSD)".into()
    }
    #[cfg(not(target_os = "macos"))]
    {
        stdout_of("df", &["-T", "."]).unwrap_or_else(|| "unknown".into())
    }
}

fn tool_version(tool: &str, args: &[&str]) -> Option<(String, String)> {
    stdout_of(tool, args).map(|v| (tool.to_string(), v))
}

/// Capture the environment for a benchmark run.
pub fn capture(seed: u64) -> BenchEnvironment {
    let mut tools = Vec::new();
    if let Some(t) = tool_version("bsdiff", &[]) {
        // bsdiff prints usage; keep it short.
        tools.push(("bsdiff".into(), t.1.replace("bsdiff: usage:", "present:")));
    }
    if let Some(t) = tool_version("xdelta3", &["-V"]) {
        tools.push(t);
    }
    if let Some(t) = tool_version("butler", &["--version"]) {
        tools.push(t);
    }
    let z = zstd::zstd_safe::version_number();
    tools.push((
        "zstd (linked library)".into(),
        format!("{}.{}.{}", z / 10_000, (z / 100) % 100, z % 100),
    ));

    BenchEnvironment {
        os: os_version(),
        arch: std::env::consts::ARCH.into(),
        cpu: cpu_model(),
        ram_gib: ram_gib(),
        disk: disk_type(),
        cavs_version: format!("cavs {}", env!("CARGO_PKG_VERSION")),
        tool_versions: tools,
        command: std::env::args().collect::<Vec<_>>().join(" "),
        seed,
    }
}

/// Render the environment as a Markdown table section.
pub fn markdown(env: &BenchEnvironment) -> String {
    let mut md = String::from("## Environment\n\n| | |\n|---|---|\n");
    md.push_str(&format!("| OS | {} ({}) |\n", env.os, env.arch));
    md.push_str(&format!("| CPU | {} |\n", env.cpu));
    md.push_str(&format!(
        "| RAM | {} |\n",
        env.ram_gib
            .map(|g| format!("{g:.0} GiB"))
            .unwrap_or_else(|| "unknown".into())
    ));
    md.push_str(&format!("| Disk | {} |\n", env.disk));
    md.push_str(&format!("| CAVS | {} |\n", env.cavs_version));
    for (tool, version) in &env.tool_versions {
        md.push_str(&format!("| {tool} | {version} |\n"));
    }
    md.push_str(&format!("| Command | `{}` |\n", env.command));
    md.push_str(&format!("| Dataset seed | {} |\n", env.seed));
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_produces_complete_metadata() {
        let env = capture(9);
        assert!(!env.os.is_empty());
        assert!(!env.cpu.is_empty());
        assert!(!env.command.is_empty());
        assert_eq!(env.seed, 9);
        let md = markdown(&env);
        for field in [
            "| OS |",
            "| CPU |",
            "| RAM |",
            "| Disk |",
            "| Command |",
            "| Dataset seed | 9 |",
        ] {
            assert!(md.contains(field), "missing {field} in:\n{md}");
        }
    }
}
