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
        Self {
            skipped_properties: s.skipped_properties,
            unsupported_types: s.unsupported_types,
            subsection_failures: s.subsection_failures,
            diags: s
                .diags
                .iter()
                .map(|d| DiagDto {
                    code: d.code.clone(),
                    message: d.message.clone(),
                })
                .collect(),
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
