/**
 * Save parser（Phase3）。
 * フル GVAS 解析は未完了。メタ Snapshot（ワールド検出・プレイヤー .sav 一覧）を返す。
 */

export type SaveParserStatus = "ok" | "partial" | "unsupported" | "missing";

export interface FileMeta {
  path: string;
  size: number;
  magic: string;
  modifiedUnix?: number | null;
}

export interface PlayerFile {
  fileName: string;
  size: number;
  magic: string;
}

export interface MapMarker {
  id: string;
  label: string;
  x: number;
  y: number;
}

export interface PalworldWorldSnapshot {
  timestamp: number;
  worldDir: string;
  levelSav?: FileMeta | null;
  levelMetaSav?: FileMeta | null;
  worldOptionSav?: FileMeta | null;
  players: PlayerFile[];
  pals: unknown[];
  guilds: unknown[];
  bases: unknown[];
  mapHints: {
    note: string;
    playerMarkers: MapMarker[];
    baseMarkers: MapMarker[];
  };
  fullParse: string;
}

export interface SaveParseResult {
  status: SaveParserStatus;
  snapshot?: PalworldWorldSnapshot;
  message?: string;
}

/** フロント単体用の degraded。実データは Tauri `save_parser_status` を使う。 */
export function parsePalworldSave(_savedDir: string): SaveParseResult {
  return {
    status: "unsupported",
    message:
      "フル Save 解析はデスクトップ側 save_parser を使用（PlZ+GVAS の Character ツリーは未実装）。",
  };
}
