/**
 * Palworld REST API クライアント。
 * RCON は非サポート。
 */

export class PalworldApiError extends Error {
  readonly statusCode: number | null;

  constructor(message: string, statusCode: number | null = null) {
    super(message);
    this.name = "PalworldApiError";
    this.statusCode = statusCode;
  }
}

export interface PalworldRestClientOptions {
  /** 例: http://localhost:8212/v1/api */
  baseUrl: string;
  username?: string;
  password: string;
  timeoutMs?: number;
}

export class PalworldRestClient {
  private readonly baseUrl: string;
  private readonly authHeader: string;
  private readonly timeoutMs: number;

  constructor(options: PalworldRestClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    const user = options.username ?? "admin";
    this.authHeader = `Basic ${btoa(`${user}:${options.password}`)}`;
    this.timeoutMs = options.timeoutMs ?? 10_000;
  }

  private async request(
    method: string,
    path: string,
    jsonBody?: Record<string, unknown>,
  ): Promise<unknown> {
    const normalized = path.startsWith("/") ? path : `/${path}`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);

    try {
      const response = await fetch(`${this.baseUrl}${normalized}`, {
        method,
        headers: {
          Accept: "application/json",
          Authorization: this.authHeader,
          ...(jsonBody ? { "Content-Type": "application/json" } : {}),
        },
        body: jsonBody ? JSON.stringify(jsonBody) : undefined,
        signal: controller.signal,
      });

      if (response.status === 401) {
        throw new PalworldApiError(
          "REST API 認証に失敗しました（ユーザー名/パスワードを確認）。",
          401,
        );
      }

      if (response.status >= 400) {
        const detail = (await response.text()).trim() || response.statusText;
        throw new PalworldApiError(
          `REST API エラー (${response.status}): ${detail}`,
          response.status,
        );
      }

      if (response.status === 204) {
        return null;
      }

      const text = await response.text();
      if (!text) {
        return null;
      }

      try {
        return JSON.parse(text) as unknown;
      } catch {
        throw new PalworldApiError(
          "REST API の応答が JSON ではありません。",
          response.status,
        );
      }
    } catch (err) {
      if (err instanceof PalworldApiError) {
        throw err;
      }
      throw new PalworldApiError(
        `REST API 接続に失敗しました: ${err instanceof Error ? err.message : String(err)}`,
      );
    } finally {
      clearTimeout(timer);
    }
  }

  async getPlayers(): Promise<Record<string, unknown>[]> {
    const data = await this.request("GET", "/players");
    if (!data) {
      return [];
    }
    if (typeof data === "object" && data !== null && "players" in data) {
      const players = (data as { players: unknown }).players;
      if (!Array.isArray(players)) {
        throw new PalworldApiError("players 応答の形式が不正です。");
      }
      return players as Record<string, unknown>[];
    }
    if (Array.isArray(data)) {
      return data as Record<string, unknown>[];
    }
    throw new PalworldApiError("players 応答の形式が不正です。");
  }

  async getInfo(): Promise<Record<string, unknown>> {
    const data = await this.request("GET", "/info");
    if (typeof data !== "object" || data === null) {
      throw new PalworldApiError("info 応答の形式が不正です。");
    }
    return data as Record<string, unknown>;
  }

  async getMetrics(): Promise<Record<string, unknown>> {
    const data = await this.request("GET", "/metrics");
    if (typeof data !== "object" || data === null) {
      throw new PalworldApiError("metrics 応答の形式が不正です。");
    }
    return data as Record<string, unknown>;
  }

  async getSettings(): Promise<Record<string, unknown>> {
    const data = await this.request("GET", "/settings");
    if (typeof data !== "object" || data === null) {
      throw new PalworldApiError("settings 応答の形式が不正です。");
    }
    return data as Record<string, unknown>;
  }

  async kick(userid: string, message: string): Promise<unknown> {
    return this.request("POST", "/kick", { userid, message });
  }

  async ban(userid: string, message: string): Promise<unknown> {
    return this.request("POST", "/ban", { userid, message });
  }

  async unban(userid: string): Promise<unknown> {
    return this.request("POST", "/unban", { userid });
  }

  async announce(message: string): Promise<unknown> {
    return this.request("POST", "/announce", { message });
  }

  async save(): Promise<unknown> {
    return this.request("POST", "/save");
  }

  async shutdown(waittime: number, message: string): Promise<unknown> {
    return this.request("POST", "/shutdown", { waittime, message });
  }

  async stop(): Promise<unknown> {
    return this.request("POST", "/stop");
  }
}

/** Core.Backup が問い合わせるバックアップ対象（SteamCMD 配置） */
export const PALWORLD_BACKUP_PATHS = ["Pal/Saved/"] as const;

/** 検出用マーカー */
export const PALWORLD_DETECT_MARKERS = [
  "PalServer.exe",
  "Pal/Binaries/Win64/PalServer-Win64-Shipping.exe",
] as const;

export { PALWORLD_CAPABILITIES } from "@lunatic-asylum/shared";
export * from "./save";