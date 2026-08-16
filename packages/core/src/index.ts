/**
 * Core 層の純粋ヘルパー（ゲーム名・Discord を知らない）。
 * プロセス制御・FS の実体は Tauri (Rust) 側。
 */

import type { BackupSettings } from "@lunatic-asylum/shared";

/** バックアップ間隔をミリ秒に変換する */
export function backupIntervalToMs(settings: BackupSettings): number {
  const n = Math.max(1, settings.intervalValue);
  switch (settings.intervalUnit) {
    case "minutes":
      return n * 60_000;
    case "hours":
      return n * 3_600_000;
    case "days":
      return n * 86_400_000;
    default:
      return n * 3_600_000;
  }
}

/** 保持数を超えた古いエントリを落とす（新しい順前提） */
export function trimKeepNewest<T>(items: T[], keepCount: number): T[] {
  const keep = Math.max(0, keepCount);
  if (items.length <= keep) {
    return items;
  }
  return items.slice(0, keep);
}

export const DEFAULT_BACKUP_SETTINGS: BackupSettings = {
  enabled: false,
  intervalValue: 6,
  intervalUnit: "hours",
  keepCount: 5,
};
