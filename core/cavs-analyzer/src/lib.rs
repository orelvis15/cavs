//! CAVS build/update analysis: a SteamPipe-style fixed-chunk update model,
//! pack-file diagnostics (heatmaps, churn, entropy, similarity) and a
//! recommendation engine for patch-friendly build layouts.
//!
//! Everything here is a *predictive model based on public documentation* —
//! never Valve's exact SteamPipe implementation, and the reports say so
//! (see [`ESTIMATE_NOTE`]).

pub mod compare;
pub mod detect;
pub mod entropy;
pub mod steampipe;
pub mod walk;
pub mod windows;

/// Mandatory labeling for every SteamPipe-style report.
pub const ESTIMATE_NOTE: &str = "SteamPipe-style estimate based on public documentation. \
     This is not Valve's exact SteamPipe implementation.";

/// Pack-file extensions across the major engines (case-insensitive).
pub const PACK_EXTS: &[&str] = &[
    "pak", "ucas", "utoc", "pck", "bundle", "assets", "ress", "archive", "big", "dat", "blob",
    "pack", "zip",
];

/// Engine hint for tailored recommendations.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Engine {
    Auto,
    Unreal,
    Unity,
    Godot,
    Custom,
}

impl Engine {
    pub fn name(self) -> &'static str {
        match self {
            Engine::Auto => "auto",
            Engine::Unreal => "unreal",
            Engine::Unity => "unity",
            Engine::Godot => "godot",
            Engine::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "unreal" => Engine::Unreal,
            "unity" => Engine::Unity,
            "godot" => Engine::Godot,
            "custom" | "generic" => Engine::Custom,
            _ => Engine::Auto,
        }
    }
}

/// True when the path looks like an engine pack file.
pub fn is_pack(path: &str) -> bool {
    match path.rsplit('.').next() {
        Some(ext) => PACK_EXTS.iter().any(|e| e.eq_ignore_ascii_case(ext)),
        None => false,
    }
}

/// Engine a path's extension belongs to, when recognizable.
pub fn engine_of(path: &str) -> Option<Engine> {
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "pak" | "ucas" | "utoc" => Some(Engine::Unreal),
        "bundle" | "assets" | "ress" => Some(Engine::Unity),
        "pck" => Some(Engine::Godot),
        _ => None,
    }
}

/// B/KiB/MiB/GiB formatter shared by the analyzer reports.
pub fn human_bytes(n: u64) -> String {
    const UNITS: &[(&str, u64)] = &[("GiB", 1 << 30), ("MiB", 1 << 20), ("KiB", 1 << 10)];
    for (unit, size) in UNITS {
        if n >= *size {
            return format!("{:.2} {unit}", n as f64 / *size as f64);
        }
    }
    format!("{n} B")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_detection() {
        assert!(is_pack("Content/Packs/world.pak"));
        assert!(is_pack("game.PCK"));
        assert!(!is_pack("readme.txt"));
        assert!(!is_pack("no_extension"));
    }

    #[test]
    fn engine_detection() {
        assert_eq!(engine_of("a/b.pck"), Some(Engine::Godot));
        assert_eq!(engine_of("a/b.pak"), Some(Engine::Unreal));
        assert_eq!(engine_of("a/b.bundle"), Some(Engine::Unity));
        assert_eq!(engine_of("a/b.txt"), None);
    }

    #[test]
    fn human_units() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2 << 20), "2.00 MiB");
    }
}
