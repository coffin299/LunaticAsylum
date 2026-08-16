/**
 * Minecraft Provider（次期実装の stub）。
 */

import {
  MINECRAFT_CAPABILITIES,
  type GameCapabilities,
} from "@lunatic-asylum/shared";

export const MINECRAFT_DETECT_MARKERS = [
  "bedrock_server.exe",
  "paper.jar",
  "spigot.jar",
  "server.jar",
] as const;

export { MINECRAFT_CAPABILITIES };

export function detectMinecraft(dirFiles: string[]): boolean {
  const lower = new Set(dirFiles.map((f) => f.toLowerCase()));
  return MINECRAFT_DETECT_MARKERS.some((m) => lower.has(m.toLowerCase()));
}

export function getMinecraftCapabilities(): GameCapabilities {
  return MINECRAFT_CAPABILITIES;
}

export const MINECRAFT_STATUS = {
  implemented: false,
  message: "Minecraft Provider is planned for a later release.",
} as const;
