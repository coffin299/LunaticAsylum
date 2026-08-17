//! Level.sav → ドメイン Snapshot

use super::base::extract_bases;
use super::character::extract_characters;
use super::container::{decompress_sav, ContainerKind};
use super::error::{ParseError, ParseStats};
use super::group::extract_guilds;
use super::gvas::read_gvas;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorldParseResult {
    pub ok: bool,
    pub container: String,
    pub save_type: u8,
    pub class_name: String,
    pub root_keys: Vec<String>,
    pub players: Vec<serde_json::Value>,
    pub pals: Vec<serde_json::Value>,
    pub guilds: Vec<serde_json::Value>,
    pub bases: Vec<serde_json::Value>,
    pub stats: StatsDto,
    pub message: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatsDto {
    pub skipped_properties: u64,
    pub unsupported_types: u64,
    pub subsection_failures: u64,
    pub diags: Vec<DiagDto>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiagDto {
    pub code: String,
    pub message: String,
}

impl From<&ParseStats> for StatsDto {
    fn from(s: &ParseStats) -> Self {
        let mut diags: Vec<DiagDto> = s
            .diags
            .iter()
            .map(|d| DiagDto {
                code: d.code.clone(),
                message: d.message.clone(),
            })
            .collect();
        if s.diag_overflow > 0 {
            diags.push(DiagDto {
                code: "diag_truncated".into(),
                message: format!("additional {} diagnostics omitted", s.diag_overflow),
            });
        }
        Self {
            skipped_properties: s.skipped_properties,
            unsupported_types: s.unsupported_types,
            subsection_failures: s.subsection_failures,
            diags,
        }
    }
}

pub fn parse_level_sav_bytes(data: &[u8]) -> Result<WorldParseResult, ParseError> {
    let dec = decompress_sav(data)?;
    let gvas = read_gvas(&dec.gvas)?;
    let mut stats = gvas.stats.clone();

    let world = gvas.world_save_data.as_ref();
    let (players, pals) = extract_characters(world, &mut stats);
    let guilds = extract_guilds(world, &mut stats);
    let bases = extract_bases(world, &mut stats);

    let container = match dec.kind {
        ContainerKind::PlZ => "PlZ",
        ContainerKind::PlM => "PlM",
        ContainerKind::CnkThenPlZ => "CNK+PlZ",
        ContainerKind::CnkThenPlM => "CNK+PlM",
    };

    let message = format!(
        "container={} class={} players={} pals={} guilds={} bases={}",
        container,
        gvas.header.save_game_class_name,
        players.len(),
        pals.len(),
        guilds.len(),
        bases.len()
    );

    Ok(WorldParseResult {
        ok: true,
        container: container.into(),
        save_type: dec.save_type,
        class_name: gvas.header.save_game_class_name,
        root_keys: gvas.root_keys,
        players,
        pals,
        guilds,
        bases,
        stats: StatsDto::from(&stats),
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_level_sav_bytes;
    use std::fs;

    #[test]
    fn e2e_level_sav_from_env() {
        let Ok(path) = std::env::var("LUNATIC_ASYLUM_LEVEL_SAV") else {
            return;
        };
        let bytes = fs::read(&path).expect("read Level.sav");
        let r = parse_level_sav_bytes(&bytes).expect("parse Level.sav");
        assert!(
            r.players.len() >= 1,
            "expected players, got {} diags={:?}",
            r.players.len(),
            r.stats.diags
        );
        assert!(
            r.players.iter().all(|p| p.get("key").and_then(|v| v.as_str()) != Some("unknown")),
            "player keys should not be unknown: {:?}",
            r.players
        );
        let mismatch = r
            .stats
            .diags
            .iter()
            .filter(|d| d.code == "size_mismatch")
            .count();
        assert_eq!(mismatch, 0, "size_mismatch should be gone");
        eprintln!("{}", r.message);
        eprintln!(
            "guilds={} bases={} skipped={} failures={} diags={}",
            r.guilds.len(),
            r.bases.len(),
            r.stats.skipped_properties,
            r.stats.subsection_failures,
            r.stats.diags.len()
        );
        for d in &r.stats.diags {
            eprintln!("{}: {}", d.code, d.message);
        }
    }
}
