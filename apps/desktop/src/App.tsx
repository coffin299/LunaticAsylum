import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { LocalePreference, ThemePreference } from "@lunatic-asylum/shared";
import { applyTheme, initTheme, saveThemePreference } from "./theme";
import {
  readLocalePreference,
  resolveLocale,
  saveLocalePreference,
} from "./i18n";
import i18n from "./i18n";
import "./styles.css";

interface ServerRow {
  id: string;
  displayName: string;
  path: string;
  providerId: string;
  status: string;
  updateAvailable?: boolean;
  pid?: number | null;
  discordRunning?: boolean;
}

interface BackupRow {
  name: string;
  path: string;
  createdAt: string;
}

interface InstanceConfig {
  restBaseUrl: string;
  restUsername: string;
  restPassword: string;
  restPasswordSet: boolean;
  backup: {
    enabled: boolean;
    intervalValue: number;
    intervalUnit: string;
    keepCount: number;
  };
  crashRestartEnabled: boolean;
  updateCheck: {
    pollingEnabled: boolean;
    intervalMinutes: number;
    autoApply: boolean;
    autoApplyOnlyWhenEmpty: boolean;
  };
  discord: {
    enabled: boolean;
    token: string;
    tokenSet: boolean;
    guildId: string;
    channelId: string;
    adminIds: string;
    pollIntervalSeconds: number;
    topicTemplate: string;
    notify: {
      joinLeave: boolean;
      restStatus: boolean;
      topic: boolean;
    };
  };
}

type Page = "servers" | "settings";

function defaultConfig(): InstanceConfig {
  return {
    restBaseUrl: "http://127.0.0.1:8212/v1/api",
    restUsername: "admin",
    restPassword: "",
    restPasswordSet: false,
    backup: {
      enabled: false,
      intervalValue: 6,
      intervalUnit: "hours",
      keepCount: 5,
    },
    crashRestartEnabled: false,
    updateCheck: {
      pollingEnabled: true,
      intervalMinutes: 180,
      autoApply: false,
      autoApplyOnlyWhenEmpty: true,
    },
    discord: {
      enabled: false,
      token: "",
      tokenSet: false,
      guildId: "",
      channelId: "",
      adminIds: "",
      pollIntervalSeconds: 15,
      topicTemplate: "Online: {current}/{max}",
      notify: {
        joinLeave: true,
        restStatus: true,
        topic: true,
      },
    },
  };
}

function App() {
  const { t } = useTranslation();
  const [page, setPage] = useState<Page>("servers");
  const [servers, setServers] = useState<ServerRow[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [appRoot, setAppRoot] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [config, setConfig] = useState<InstanceConfig | null>(null);
  const [backups, setBackups] = useState<BackupRow[]>([]);
  const [logTail, setLogTail] = useState("");
  const [players, setPlayers] = useState<Record<string, unknown>[]>([]);
  const [metrics, setMetrics] = useState<Record<string, unknown> | null>(null);
  const [info, setInfo] = useState<Record<string, unknown> | null>(null);
  const [announce, setAnnounce] = useState("");
  const [restoreArmed, setRestoreArmed] = useState<string | null>(null);
  const [saveStatus, setSaveStatus] = useState<string>("");
  const [saveSnapshot, setSaveSnapshot] = useState<Record<string, unknown> | null>(null);
  const [dashMetrics, setDashMetrics] = useState<Record<string, unknown> | null>(null);
  const [unbanId, setUnbanId] = useState("");
  const [discordRunning, setDiscordRunning] = useState(false);
  const [theme, setTheme] = useState<ThemePreference>(() => initTheme());
  const [locale, setLocale] = useState<LocalePreference>(() => readLocalePreference());

  const selected = useMemo(
    () => servers.find((s) => s.id === selectedId) ?? null,
    [servers, selectedId],
  );

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke<string>("ensure_servers_layout");
      const root = await invoke<string>("get_app_root");
      const list = await invoke<ServerRow[]>("list_server_instances");
      setAppRoot(root);
      setServers(list);
      if (selectedId && !list.some((s) => s.id === selectedId)) {
        setSelectedId(null);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [selectedId]);

  const loadDetail = useCallback(async (id: string) => {
    try {
      const cfg = await invoke<InstanceConfig>("read_instance_config", { id });
      setConfig({ ...defaultConfig(), ...cfg, discord: { ...defaultConfig().discord, ...cfg.discord, notify: { ...defaultConfig().discord.notify, ...cfg.discord?.notify } }, updateCheck: { ...defaultConfig().updateCheck, ...cfg.updateCheck }, backup: { ...defaultConfig().backup, ...cfg.backup } });
      await invoke("set_crash_restart", { id, enabled: cfg.crashRestartEnabled });
      const b = await invoke<BackupRow[]>("list_backups", { id });
      setBackups(b);
      const log = await invoke<string>("read_log_tail", { id, maxBytes: 32_000 });
      setLogTail(log);
      const save = await invoke<{
        status: string;
        message?: string;
        snapshot?: Record<string, unknown>;
      }>("save_parser_status", { id });
      setSaveStatus(save.message ?? save.status);
      setSaveSnapshot(save.snapshot ?? null);
      try {
        const dash = await invoke<{
          metrics?: Record<string, unknown>;
          info?: Record<string, unknown>;
        }>("dashboard_metrics", { id });
        setDashMetrics(dash.metrics ?? null);
      } catch {
        setDashMetrics(null);
      }
      const d = await invoke<boolean>("discord_integration_status", { id });
      setDiscordRunning(d);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen("servers-changed", () => {
      void refresh();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [refresh]);

  useEffect(() => {
    if (selectedId) {
      void loadDetail(selectedId);
    }
  }, [selectedId, loadDetail]);

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await fn();
      await refresh();
      if (selectedId) {
        await loadDetail(selectedId);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const refreshRest = async () => {
    if (!selectedId) {
      return;
    }
    try {
      const [p, m, i] = await Promise.all([
        invoke<Record<string, unknown>[]>("rest_get_players", { id: selectedId }),
        invoke<Record<string, unknown>>("rest_get_metrics", { id: selectedId }),
        invoke<Record<string, unknown>>("rest_get_info", { id: selectedId }),
      ]);
      setPlayers(p);
      setMetrics(m);
      setInfo(i);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const onThemeChange = (value: ThemePreference) => {
    setTheme(value);
    saveThemePreference(value);
    applyTheme(value);
  };

  const onLocaleChange = (value: LocalePreference) => {
    setLocale(value);
    saveLocalePreference(value);
    void i18n.changeLanguage(resolveLocale(value));
  };

  const providerLabel = (id: string) => {
    if (id === "palworld") return t("servers.palworld");
    if (id === "minecraft") return t("servers.minecraft");
    return t("servers.unknown");
  };

  const statusLabel = (status: string) => {
    if (status === "running") return t("servers.running");
    if (status === "installing") return t("servers.installing");
    return t("servers.stopped");
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">{t("appName")}</div>
        <nav className="nav">
          <button
            type="button"
            className={page === "servers" ? "active" : ""}
            onClick={() => setPage("servers")}
          >
            {t("nav.servers")}
          </button>
          <button
            type="button"
            className={page === "settings" ? "active" : ""}
            onClick={() => setPage("settings")}
          >
            {t("nav.settings")}
          </button>
        </nav>
      </aside>

      <main className="main">
        {error && (
          <p className="muted" style={{ color: "var(--danger)" }}>
            {t("common.error")}: {error}
          </p>
        )}

        {page === "servers" && (
          <>
            <section className="panel" style={{ marginBottom: "1rem" }}>
              <h1>{t("servers.title")}</h1>
              <p className="muted">{t("servers.openFolderHint")}</p>
              <div className="toolbar">
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={busy}
                  onClick={() => void refresh()}
                >
                  {t("servers.refresh")}
                </button>
                <input
                  className="btn"
                  style={{ minWidth: 160 }}
                  placeholder={t("servers.newName")}
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                />
                <button
                  type="button"
                  className="btn"
                  disabled={busy || !newName.trim()}
                  onClick={() =>
                    void run(async () => {
                      await invoke("install_palworld", { id: newName.trim() });
                      setNewName("");
                    })
                  }
                >
                  {t("servers.installPalworld")}
                </button>
              </div>
              {loading && <p className="muted">{t("common.loading")}</p>}
              {!loading && servers.length === 0 && (
                <div className="empty">{t("servers.empty")}</div>
              )}
              {!loading && servers.length > 0 && (
                <table className="table">
                  <thead>
                    <tr>
                      <th>{t("servers.title")}</th>
                      <th>{t("servers.provider")}</th>
                      <th>{t("servers.status")}</th>
                      <th>PID</th>
                    </tr>
                  </thead>
                  <tbody>
                    {servers.map((row) => (
                      <tr
                        key={row.id}
                        onClick={() => setSelectedId(row.id)}
                        style={{
                          cursor: "pointer",
                          background:
                            selectedId === row.id ? "var(--row-hover)" : undefined,
                        }}
                      >
                        <td>
                          {row.displayName}
                          {row.updateAvailable ? (
                            <span className="badge" style={{ marginLeft: 8 }}>
                              {t("servers.updateAvailable")}
                            </span>
                          ) : null}
                        </td>
                        <td>
                          <span className="badge">{providerLabel(row.providerId)}</span>
                        </td>
                        <td>{statusLabel(row.status)}</td>
                        <td>{row.pid ?? "—"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>

            {selected && config && (
              <section className="panel">
                <h1>{selected.displayName}</h1>
                <p className="muted path">{selected.path}</p>
                {dashMetrics && (
                  <p className="muted">
                    {t("ops.playersShort")}:{" "}
                    {String(
                      dashMetrics.currentplayernum ?? "—",
                    )}
                    /
                    {String(dashMetrics.maxplayernum ?? "—")}{" "}
                    · FPS {String(dashMetrics.serverfps ?? "—")}
                    {selected.updateAvailable
                      ? ` · ${t("servers.updateAvailable")}`
                      : ""}
                  </p>
                )}
                <div className="toolbar">
                  <button
                    type="button"
                    className="btn btn-primary"
                    disabled={busy || selected.providerId !== "palworld"}
                    onClick={() => void run(async () => invoke("start_server", { id: selected.id }))}
                  >
                    {t("ops.start")}
                  </button>
                  <button
                    type="button"
                    className="btn"
                    disabled={busy}
                    onClick={() => void run(async () => invoke("stop_server", { id: selected.id }))}
                  >
                    {t("ops.stop")}
                  </button>
                  <button
                    type="button"
                    className="btn"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => invoke("restart_server", { id: selected.id }))
                    }
                  >
                    {t("ops.restart")}
                  </button>
                  <button
                    type="button"
                    className="btn"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => invoke("check_palworld_update", { id: selected.id }))
                    }
                  >
                    {t("ops.checkUpdate")}
                  </button>
                  <button
                    type="button"
                    className="btn"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => invoke("update_palworld", { id: selected.id }))
                    }
                  >
                    {t("ops.update")}
                  </button>
                  <button
                    type="button"
                    className="btn"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => {
                        await invoke("create_backup", { id: selected.id });
                      })
                    }
                  >
                    {t("ops.backupNow")}
                  </button>
                  <button type="button" className="btn" onClick={() => void refreshRest()}>
                    {t("ops.refreshRest")}
                  </button>
                </div>

                <div className="field">
                  <span>{t("ops.restUrl")}</span>
                  <input
                    className="btn"
                    value={config.restBaseUrl}
                    onChange={(e) =>
                      setConfig({ ...config, restBaseUrl: e.target.value })
                    }
                  />
                </div>
                <div className="field">
                  <span>{t("ops.restUser")}</span>
                  <input
                    className="btn"
                    value={config.restUsername}
                    onChange={(e) =>
                      setConfig({ ...config, restUsername: e.target.value })
                    }
                  />
                </div>
                <div className="field">
                  <span>
                    {t("ops.restPassword")}
                    {config.restPasswordSet ? ` (${t("ops.secretSaved")})` : ""}
                  </span>
                  <input
                    className="btn"
                    type="password"
                    autoComplete="new-password"
                    placeholder={
                      config.restPasswordSet
                        ? t("ops.secretPlaceholder")
                        : ""
                    }
                    value={config.restPassword}
                    onChange={(e) =>
                      setConfig({ ...config, restPassword: e.target.value })
                    }
                  />
                </div>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.crashRestartEnabled}
                      onChange={(e) =>
                        setConfig({ ...config, crashRestartEnabled: e.target.checked })
                      }
                    />{" "}
                    {t("ops.crashRestart")}
                  </span>
                </label>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.backup.enabled}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          backup: { ...config.backup, enabled: e.target.checked },
                        })
                      }
                    />{" "}
                    {t("ops.backupEnabled")}
                  </span>
                </label>
                <div className="toolbar">
                  <label className="field" style={{ flex: 1 }}>
                    <span>{t("ops.backupInterval")}</span>
                    <input
                      className="btn"
                      type="number"
                      min={1}
                      value={config.backup.intervalValue}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          backup: {
                            ...config.backup,
                            intervalValue: Number(e.target.value) || 1,
                          },
                        })
                      }
                    />
                  </label>
                  <select
                    className="btn"
                    value={config.backup.intervalUnit}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        backup: { ...config.backup, intervalUnit: e.target.value },
                      })
                    }
                  >
                    <option value="minutes">{t("ops.minutes")}</option>
                    <option value="hours">{t("ops.hours")}</option>
                    <option value="days">{t("ops.days")}</option>
                  </select>
                  <label className="field">
                    <span>{t("ops.backupKeep")}</span>
                    <input
                      className="btn"
                      type="number"
                      min={1}
                      value={config.backup.keepCount}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          backup: {
                            ...config.backup,
                            keepCount: Number(e.target.value) || 1,
                          },
                        })
                      }
                    />
                  </label>
                </div>

                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.updateCheck.pollingEnabled}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          updateCheck: {
                            ...config.updateCheck,
                            pollingEnabled: e.target.checked,
                          },
                        })
                      }
                    />{" "}
                    {t("ops.updatePolling")}
                  </span>
                </label>
                <div className="field">
                  <span>{t("ops.updateInterval")}</span>
                  <input
                    className="btn"
                    type="number"
                    min={15}
                    value={config.updateCheck.intervalMinutes}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        updateCheck: {
                          ...config.updateCheck,
                          intervalMinutes: Number(e.target.value) || 15,
                        },
                      })
                    }
                  />
                </div>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.updateCheck.autoApply}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          updateCheck: {
                            ...config.updateCheck,
                            autoApply: e.target.checked,
                          },
                        })
                      }
                    />{" "}
                    {t("ops.updateAutoApply")}
                  </span>
                </label>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.updateCheck.autoApplyOnlyWhenEmpty}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          updateCheck: {
                            ...config.updateCheck,
                            autoApplyOnlyWhenEmpty: e.target.checked,
                          },
                        })
                      }
                    />{" "}
                    {t("ops.updateOnlyEmpty")}
                  </span>
                </label>

                <h2 style={{ marginTop: "1.5rem", fontSize: "1.05rem" }}>
                  {t("discord.title")}
                </h2>
                <p className="muted">{t("discord.hint")}</p>
                <p className="muted">
                  {discordRunning ? t("discord.running") : t("discord.stopped")}
                </p>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.discord.enabled}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          discord: { ...config.discord, enabled: e.target.checked },
                        })
                      }
                    />{" "}
                    {t("discord.enabled")}
                  </span>
                </label>
                <div className="field">
                  <span>
                    {t("discord.token")}
                    {config.discord.tokenSet ? ` (${t("ops.secretSaved")})` : ""}
                  </span>
                  <input
                    className="btn"
                    type="password"
                    autoComplete="new-password"
                    placeholder={
                      config.discord.tokenSet
                        ? t("ops.secretPlaceholder")
                        : ""
                    }
                    value={config.discord.token}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        discord: { ...config.discord, token: e.target.value },
                      })
                    }
                  />
                </div>
                <div className="field">
                  <span>{t("discord.guildId")}</span>
                  <input
                    className="btn"
                    value={config.discord.guildId}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        discord: { ...config.discord, guildId: e.target.value },
                      })
                    }
                  />
                </div>
                <div className="field">
                  <span>{t("discord.channelId")}</span>
                  <input
                    className="btn"
                    value={config.discord.channelId}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        discord: { ...config.discord, channelId: e.target.value },
                      })
                    }
                  />
                </div>
                <div className="field">
                  <span>{t("discord.adminIds")}</span>
                  <input
                    className="btn"
                    value={config.discord.adminIds}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        discord: { ...config.discord, adminIds: e.target.value },
                      })
                    }
                  />
                </div>
                <div className="field">
                  <span>{t("discord.pollInterval")}</span>
                  <input
                    className="btn"
                    type="number"
                    min={5}
                    value={config.discord.pollIntervalSeconds}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        discord: {
                          ...config.discord,
                          pollIntervalSeconds: Number(e.target.value) || 5,
                        },
                      })
                    }
                  />
                </div>
                <div className="field">
                  <span>{t("discord.topicTemplate")}</span>
                  <input
                    className="btn"
                    value={config.discord.topicTemplate}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        discord: {
                          ...config.discord,
                          topicTemplate: e.target.value,
                        },
                      })
                    }
                  />
                </div>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.discord.notify.joinLeave}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          discord: {
                            ...config.discord,
                            notify: {
                              ...config.discord.notify,
                              joinLeave: e.target.checked,
                            },
                          },
                        })
                      }
                    />{" "}
                    {t("discord.notifyJoinLeave")}
                  </span>
                </label>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.discord.notify.restStatus}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          discord: {
                            ...config.discord,
                            notify: {
                              ...config.discord.notify,
                              restStatus: e.target.checked,
                            },
                          },
                        })
                      }
                    />{" "}
                    {t("discord.notifyRest")}
                  </span>
                </label>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.discord.notify.topic}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          discord: {
                            ...config.discord,
                            notify: {
                              ...config.discord.notify,
                              topic: e.target.checked,
                            },
                          },
                        })
                      }
                    />{" "}
                    {t("discord.notifyTopic")}
                  </span>
                </label>
                <div className="toolbar">
                  <button
                    type="button"
                    className="btn btn-primary"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => {
                        await invoke("write_instance_config", {
                          id: selected.id,
                          config,
                        });
                      })
                    }
                  >
                    {t("ops.saveConfig")}
                  </button>
                  <button
                    type="button"
                    className="btn"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => {
                        await invoke("write_instance_config", {
                          id: selected.id,
                          config,
                        });
                        await invoke("apply_discord_integration", {
                          id: selected.id,
                        });
                      })
                    }
                  >
                    {t("discord.apply")}
                  </button>
                  <button
                    type="button"
                    className="btn"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => {
                        await invoke("stop_discord_integration", {
                          id: selected.id,
                        });
                      })
                    }
                  >
                    {t("discord.stop")}
                  </button>
                </div>

                <h2 style={{ marginTop: "1.5rem", fontSize: "1.05rem" }}>
                  {t("ops.rest")}
                </h2>
                {info && (
                  <pre className="path" style={{ whiteSpace: "pre-wrap" }}>
                    {JSON.stringify(info, null, 2)}
                  </pre>
                )}
                {metrics && (
                  <pre className="path" style={{ whiteSpace: "pre-wrap" }}>
                    {JSON.stringify(metrics, null, 2)}
                  </pre>
                )}
                <div className="toolbar">
                  <input
                    className="btn"
                    style={{ flex: 1 }}
                    value={announce}
                    placeholder={t("ops.announce")}
                    onChange={(e) => setAnnounce(e.target.value)}
                  />
                  <button
                    type="button"
                    className="btn"
                    onClick={() =>
                      void run(async () => {
                        await invoke("rest_announce", {
                          id: selected.id,
                          message: announce,
                        });
                        setAnnounce("");
                      })
                    }
                  >
                    {t("ops.send")}
                  </button>
                  <button
                    type="button"
                    className="btn"
                    onClick={() =>
                      void run(async () => {
                        await invoke("rest_save", { id: selected.id });
                      })
                    }
                  >
                    Save
                  </button>
                  <button
                    type="button"
                    className="btn"
                    onClick={() =>
                      void run(async () => {
                        const s = await invoke<Record<string, unknown>>(
                          "rest_get_settings",
                          { id: selected.id },
                        );
                        setInfo(s);
                      })
                    }
                  >
                    Settings
                  </button>
                </div>
                <div className="toolbar">
                  <input
                    className="btn"
                    style={{ flex: 1 }}
                    value={unbanId}
                    placeholder="unban userId"
                    onChange={(e) => setUnbanId(e.target.value)}
                  />
                  <button
                    type="button"
                    className="btn"
                    disabled={!unbanId.trim()}
                    onClick={() =>
                      void run(async () => {
                        await invoke("rest_unban", {
                          id: selected.id,
                          userid: unbanId.trim(),
                        });
                        setUnbanId("");
                      })
                    }
                  >
                    Unban
                  </button>
                  <button
                    type="button"
                    className="btn"
                    onClick={() => {
                      if (
                        !window.confirm(
                          "REST shutdown（60秒）を実行しますか？",
                        )
                      ) {
                        return;
                      }
                      void run(async () => {
                        await invoke("rest_shutdown", {
                          id: selected.id,
                          waittime: 60,
                          message: "Server is shutting down",
                        });
                      });
                    }}
                  >
                    Shutdown
                  </button>
                </div>
                <table className="table">
                  <thead>
                    <tr>
                      <th>Player</th>
                      <th>userID</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {players.map((p, idx) => {
                      const name = String(p.name ?? p.playerName ?? "?");
                      const userid = String(p.userId ?? p.userid ?? "");
                      return (
                        <tr key={`${userid}-${idx}`}>
                          <td>{name}</td>
                          <td className="path">{userid}</td>
                          <td>
                            <button
                              type="button"
                              className="btn"
                              onClick={() =>
                                void run(async () => {
                                  await invoke("rest_kick", {
                                    id: selected.id,
                                    userid,
                                    message: "kicked by LunaticAsylum",
                                  });
                                })
                              }
                            >
                              Kick
                            </button>{" "}
                            <button
                              type="button"
                              className="btn"
                              onClick={() =>
                                void run(async () => {
                                  await invoke("rest_ban", {
                                    id: selected.id,
                                    userid,
                                    message: "banned by LunaticAsylum",
                                  });
                                })
                              }
                            >
                              Ban
                            </button>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>

                <h2 style={{ marginTop: "1.5rem", fontSize: "1.05rem" }}>
                  {t("ops.backups")}
                </h2>
                <table className="table">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {backups.map((b) => (
                      <tr key={b.name}>
                        <td>{b.name}</td>
                        <td>
                          {restoreArmed === b.name ? (
                            <button
                              type="button"
                              className="btn btn-primary"
                              onClick={() =>
                                void run(async () => {
                                  await invoke("restore_backup", {
                                    id: selected.id,
                                    backupName: b.name,
                                  });
                                  setRestoreArmed(null);
                                })
                              }
                            >
                              {t("ops.confirmRestore")}
                            </button>
                          ) : (
                            <button
                              type="button"
                              className="btn"
                              onClick={() => setRestoreArmed(b.name)}
                            >
                              {t("ops.restore")}
                            </button>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>

                <h2 style={{ marginTop: "1.5rem", fontSize: "1.05rem" }}>
                  {t("ops.saveParser")}
                </h2>
                <p className="muted">{saveStatus || "—"}</p>
                {saveSnapshot && (
                  <pre
                    className="path"
                    style={{
                      maxHeight: 200,
                      overflow: "auto",
                      whiteSpace: "pre-wrap",
                      background: "var(--bg)",
                      padding: "0.75rem",
                      border: "1px solid var(--border)",
                    }}
                  >
                    {JSON.stringify(saveSnapshot, null, 2)}
                  </pre>
                )}

                <h2 style={{ marginTop: "1.5rem", fontSize: "1.05rem" }}>
                  {t("ops.map")}
                </h2>
                <p className="muted">{t("ops.mapSoon")}</p>
                <div
                  className="map-plane"
                  style={{
                    height: 220,
                    border: "1px solid var(--border)",
                    background: "var(--bg)",
                    position: "relative",
                    overflow: "hidden",
                  }}
                >
                  <div
                    style={{
                      position: "absolute",
                      inset: 0,
                      display: "grid",
                      placeItems: "center",
                      color: "var(--text-muted)",
                      fontSize: "0.9rem",
                      padding: "1rem",
                      textAlign: "center",
                    }}
                  >
                    {(saveSnapshot?.mapHints as { note?: string } | undefined)
                      ?.note ?? t("ops.mapSoon")}
                  </div>
                </div>

                <h2 style={{ marginTop: "1.5rem", fontSize: "1.05rem" }}>
                  {t("ops.logs")}
                </h2>
                <pre
                  className="path"
                  style={{
                    maxHeight: 240,
                    overflow: "auto",
                    background: "var(--bg)",
                    padding: "0.75rem",
                    border: "1px solid var(--border)",
                    borderRadius: 6,
                  }}
                >
                  {logTail || "—"}
                </pre>
              </section>
            )}
          </>
        )}

        {page === "settings" && (
          <section className="panel">
            <h1>{t("settings.title")}</h1>
            <label className="field">
              <span>{t("settings.language")}</span>
              <select
                value={locale}
                onChange={(e) => onLocaleChange(e.target.value as LocalePreference)}
              >
                <option value="system">{t("settings.system")}</option>
                <option value="ja">{t("settings.japanese")}</option>
                <option value="en">{t("settings.english")}</option>
              </select>
            </label>
            <label className="field">
              <span>{t("settings.theme")}</span>
              <select
                value={theme}
                onChange={(e) => onThemeChange(e.target.value as ThemePreference)}
              >
                <option value="system">{t("settings.system")}</option>
                <option value="light">{t("settings.light")}</option>
                <option value="dark">{t("settings.dark")}</option>
              </select>
            </label>
            <div className="field">
              <span>{t("settings.appRoot")}</span>
              <code className="path">{appRoot || "—"}</code>
            </div>
            <p className="muted">{t("settings.unsignedNote")}</p>
          </section>
        )}
      </main>
    </div>
  );
}

export default App;
