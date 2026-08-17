/** ゲーム横断の Capabilities・DTO */

export type ThemePreference = "system" | "light" | "dark";
export type LocalePreference = "system" | "ja" | "en";

export interface GameCapabilities {
  console: boolean;
  rcon: boolean;
  playerList: boolean;
  playerModeration: boolean;
  metrics: boolean;
  backups: boolean;
  config: boolean;
  saveParser: boolean;
  map: boolean;
  discord: boolean;
}

export type InstanceStatus = "running" | "stopped" | "unknown" | "installing";

export type ProviderId = "palworld" | "minecraft" | "unknown";

export type MinecraftServerType =
  | "vanilla"
  | "paper"
  | "purpur"
  | "fabric"
  | "neoforge"
  | "forge"
  | "spigot"
  | "other"
  | "unknown";

export interface ServerInstance {
  /** Servers 直下のフォルダ名 */
  id: string;
  displayName: string;
  path: string;
  providerId: ProviderId;
  status: InstanceStatus;
  capabilities: GameCapabilities;
  updateAvailable?: boolean;
}

export interface BackupSettings {
  enabled: boolean;
  intervalValue: number;
  intervalUnit: "minutes" | "hours" | "days";
  keepCount: number;
}

export interface CrashRestartSettings {
  enabled: boolean;
}

export interface UpdateCheckSettings {
  pollingEnabled: boolean;
  /** ポーリング間隔（分） */
  intervalMinutes: number;
  autoApply: boolean;
}

export const EMPTY_CAPABILITIES: GameCapabilities = {
  console: false,
  rcon: false,
  playerList: false,
  playerModeration: false,
  metrics: false,
  backups: false,
  config: false,
  saveParser: false,
  map: false,
  discord: false,
};

export const PALWORLD_CAPABILITIES: GameCapabilities = {
  console: false,
  rcon: false,
  playerList: true,
  playerModeration: true,
  metrics: true,
  backups: true,
  config: true,
  saveParser: true,
  map: true,
  discord: true,
};

export const MINECRAFT_CAPABILITIES: GameCapabilities = {
  console: true,
  rcon: true,
  playerList: true,
  playerModeration: true,
  metrics: true,
  backups: true,
  config: true,
  saveParser: false,
  map: false,
  discord: false,
};
