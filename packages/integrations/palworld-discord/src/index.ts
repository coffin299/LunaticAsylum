/**
 * Palworld 専用 Discord Integration（ゲーム横断の再利用はしない）。
 * 共通のサーバー管理は Core / Provider。ここは設定型とコマンド一覧。
 * Bot 実行は desktop（Rust）側。
 */

export interface PalworldDiscordConfig {
  enabled: boolean;
  token: string;
  guildId: string;
  channelId: string;
  adminIds: string[];
  pollIntervalSeconds: number;
  topicTemplate: string;
  restBaseUrl: string;
  restUsername: string;
  restPassword: string;
  notifyJoinLeave: boolean;
  notifyRestStatus: boolean;
  notifyTopic: boolean;
}

export const DEFAULT_PALWORLD_DISCORD_CONFIG: PalworldDiscordConfig = {
  enabled: false,
  token: "",
  guildId: "",
  channelId: "",
  adminIds: [],
  pollIntervalSeconds: 15,
  topicTemplate: "Online: {current}/{max}",
  restBaseUrl: "http://127.0.0.1:8212/v1/api",
  restUsername: "admin",
  restPassword: "",
  notifyJoinLeave: true,
  notifyRestStatus: true,
  notifyTopic: true,
};

/** スラッシュコマンド一覧（Palworld 専用） */
export const PALWORLD_DISCORD_SLASH_COMMANDS = [
  "help",
  "players",
  "info",
  "metrics",
  "settings",
  "kick",
  "ban",
  "unban",
  "announce",
  "save",
  "shutdown",
  "stop",
] as const;

export type PalworldDiscordSlashCommand =
  (typeof PALWORLD_DISCORD_SLASH_COMMANDS)[number];
