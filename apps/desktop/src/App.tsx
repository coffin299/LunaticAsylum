import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import type { LocalePreference } from "@lunatic-asylum/shared";
import {
  readLocalePreference,
  resolveLocale,
  saveLocalePreference,
} from "./i18n";
import i18n from "./i18n";
import "./styles.css";

const MC_TYPES = [
  "vanilla",
  "paper",
  "purpur",
  "fabric",
  "neoforge",
  "forge",
  "spigot",
  "other",
  "unknown",
] as const;

interface ServerRow {
  id: string;
  displayName: string;
  path: string;
  providerId: string;
  status: string;
  updateAvailable?: boolean;
  pid?: number | null;
  discordRunning?: boolean;
  launchable?: boolean;
  minecraftServerType?: string | null;
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
  restApiEnabled: boolean;
  restApiPort: number;
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
  art: { bannerPath: string };
  minecraft: {
    serverType: string;
    jarFile: string;
    jvmArgs: string;
    serverArgs: string;
  };
}

type ServerView = "overview" | "palworld-settings" | "minecraft-props";

interface HostResources {
  cpuPercent: number;
  memoryUsed: number;
  memoryTotal: number;
  diskUsed: number;
  diskTotal: number;
}

type Page = "servers" | "settings";

function defaultConfig(): InstanceConfig {
  return {
    restBaseUrl: "http://127.0.0.1:8212/v1/api",
    restUsername: "admin",
    restPassword: "",
    restPasswordSet: false,
    restApiEnabled: true,
    restApiPort: 8212,
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
    art: { bannerPath: "" },
    minecraft: {
      serverType: "unknown",
      jarFile: "",
      jvmArgs: "-Xms2G -Xmx4G",
      serverArgs: "nogui",
    },
  };
}

function mergeInstanceConfig(cfg: InstanceConfig): InstanceConfig {
  return {
    ...defaultConfig(),
    ...cfg,
    discord: {
      ...defaultConfig().discord,
      ...cfg.discord,
      notify: {
        ...defaultConfig().discord.notify,
        ...cfg.discord?.notify,
      },
    },
    updateCheck: { ...defaultConfig().updateCheck, ...cfg.updateCheck },
    backup: { ...defaultConfig().backup, ...cfg.backup },
    art: { ...defaultConfig().art, ...cfg.art },
    minecraft: { ...defaultConfig().minecraft, ...cfg.minecraft },
    restApiEnabled: cfg.restApiEnabled ?? defaultConfig().restApiEnabled,
    restApiPort: cfg.restApiPort ?? defaultConfig().restApiPort,
    restBaseUrl:
      cfg.restBaseUrl ??
      `http://127.0.0.1:${cfg.restApiPort ?? 8212}/v1/api`,
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
  const [locale, setLocale] = useState<LocalePreference>(() => readLocalePreference());
  const [serverView, setServerView] = useState<ServerView>("overview");
  const [detailLoading, setDetailLoading] = useState(false);
  const [saveLoading, setSaveLoading] = useState(false);
  const [hostResources, setHostResources] = useState<HostResources | null>(null);
  const [bannerSrc, setBannerSrc] = useState<string | null>(null);
  const [steamcmdNeeded, setSteamcmdNeeded] = useState(false);
  const [palSettingsRaw, setPalSettingsRaw] = useState("");
  const [mcPropsRaw, setMcPropsRaw] = useState("");
  const [palRunningWarning, setPalRunningWarning] = useState(false);
  const [closePrompt, setClosePrompt] = useState<string[] | null>(null);
  const [closeShuttingDown, setCloseShuttingDown] = useState(false);
  const [mcRconEnabled, setMcRconEnabled] = useState(false);
  const [mcRconPort, setMcRconPort] = useState(25575);
  const [mcRconPassword, setMcRconPassword] = useState("");
  const [mcRconPasswordSet, setMcRconPasswordSet] = useState(false);
  const [mcConsoleCmd, setMcConsoleCmd] = useState("");
  const forceCloseRef = useRef(false);

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

  const loadDetail = useCallback(async (id: string, row?: ServerRow | null) => {
    setDetailLoading(true);
    setSaveLoading(true);
    setSaveStatus("");
    setSaveSnapshot(null);
    setServerView("overview");
    try {
      const cfg = await invoke<InstanceConfig>("read_instance_config", { id });
      const merged = mergeInstanceConfig(cfg);
      if (
        row?.providerId === "minecraft" &&
        merged.minecraft.serverType === "unknown"
      ) {
        const suggested = await invoke<string>("suggest_minecraft_type", { id });
        if (suggested !== "unknown") {
          merged.minecraft.serverType = suggested;
        }
      }
      setConfig(merged);
      setDetailLoading(false);
      if (row?.providerId === "minecraft") {
        void invoke<{
          enabled: boolean;
          port: number;
          passwordConfigured: boolean;
        }>("read_minecraft_rcon_settings", { id })
          .then((r) => applyMcRconFromDisk(r))
          .catch(() => undefined);
      }
      await invoke("set_crash_restart", { id, enabled: cfg.crashRestartEnabled });

      void invoke<{ path?: string; transparent: boolean }>("get_server_banner", { id })
        .then((b) => {
          setBannerSrc(b.path && !b.transparent ? convertFileSrc(b.path) : null);
        })
        .catch(() => setBannerSrc(null));

      const [b, log, dash, d] = await Promise.all([
        invoke<BackupRow[]>("list_backups", { id }),
        invoke<string>("read_log_tail", { id, maxBytes: 32_000 }),
        invoke<{ metrics?: Record<string, unknown> }>("dashboard_metrics", { id }).catch(
          () => ({ metrics: null }),
        ),
        invoke<boolean>("discord_integration_status", { id }),
      ]);
      setBackups(b);
      setLogTail(log);
      setDashMetrics(dash.metrics ?? null);
      setDiscordRunning(d);

      void invoke<{
        status: string;
        message?: string;
        snapshot?: Record<string, unknown>;
      }>("save_parser_status", { id })
        .then((save) => {
          setSaveStatus(save.message ?? save.status);
          setSaveSnapshot(save.snapshot ?? null);
        })
        .catch((err) => {
          setSaveStatus(err instanceof Error ? err.message : String(err));
        })
        .finally(() => setSaveLoading(false));
    } catch (err) {
      setDetailLoading(false);
      setSaveLoading(false);
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
      void loadDetail(selectedId, selected);
    } else {
      setConfig(null);
      setBannerSrc(null);
    }
  }, [selectedId, loadDetail, selected]);

  useEffect(() => {
    const tick = () => {
      void invoke<HostResources>("get_host_resources")
        .then(setHostResources)
        .catch(() => setHostResources(null));
    };
    tick();
    const t = window.setInterval(tick, 3000);
    return () => window.clearInterval(t);
  }, []);

  useEffect(() => {
    void invoke<{ installed: boolean }>("steamcmd_status")
      .then((s) => setSteamcmdNeeded(!s.installed))
      .catch(() => setSteamcmdNeeded(false));
  }, [servers]);

  useEffect(() => {
    let unlistenClose: (() => void) | undefined;
    void getCurrentWindow()
      .onCloseRequested(async (event) => {
        if (forceCloseRef.current) {
          return;
        }
        const running = await invoke<string[]>("list_running_server_ids");
        if (running.length === 0) {
          return;
        }
        event.preventDefault();
        setClosePrompt(running);
      })
      .then((fn) => {
        unlistenClose = fn;
      });
    return () => {
      unlistenClose?.();
    };
  }, []);

  const confirmAppExit = async () => {
    if (closeShuttingDown) {
      return;
    }
    setCloseShuttingDown(true);
    try {
      await invoke("shutdown_all_running_servers");
      forceCloseRef.current = true;
      setClosePrompt(null);
      await getCurrentWindow().close();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setCloseShuttingDown(false);
    }
  };

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

  const refreshRemoteOps = async () => {
    if (!selectedId || !selected) {
      return;
    }
    try {
      if (selected.providerId === "palworld") {
        const [p, m, i] = await Promise.all([
          invoke<Record<string, unknown>[]>("rest_get_players", { id: selectedId }),
          invoke<Record<string, unknown>>("rest_get_metrics", { id: selectedId }),
          invoke<Record<string, unknown>>("rest_get_info", { id: selectedId }),
        ]);
        setPlayers(p);
        setMetrics(m);
        setInfo(i);
      } else if (selected.providerId === "minecraft") {
        const [p, m, i] = await Promise.all([
          invoke<Record<string, unknown>[]>("minecraft_rcon_get_players", {
            id: selectedId,
          }),
          invoke<Record<string, unknown>>("minecraft_rcon_get_metrics", {
            id: selectedId,
          }),
          invoke<Record<string, unknown>>("minecraft_rcon_get_info", { id: selectedId }),
        ]);
        setPlayers(p);
        setMetrics(m);
        setInfo(i);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const onLocaleChange = (value: LocalePreference) => {
    setLocale(value);
    saveLocalePreference(value);
    void i18n.changeLanguage(resolveLocale(value));
  };

  const formatBytes = (n: number) => {
    if (n >= 1_073_741_824) return `${(n / 1_073_741_824).toFixed(1)} GB`;
    if (n >= 1_048_576) return `${(n / 1_048_576).toFixed(0)} MB`;
    return `${n} B`;
  };

  const providerLabel = (row: ServerRow) => {
    if (row.providerId === "palworld") return t("servers.palworld");
    if (row.providerId === "minecraft") {
      const st = row.minecraftServerType ?? "unknown";
      const key = `servers.minecraftType.${st}` as const;
      return t(key, { defaultValue: `Minecraft-${st}` });
    }
    return t("servers.unknown");
  };

  const openPalworldSettings = async () => {
    if (!selectedId) return;
    const data = await invoke<{
      raw: string;
      runningWarning: boolean;
      exists: boolean;
    }>("read_palworld_settings", { id: selectedId });
    setPalSettingsRaw(data.raw);
    setPalRunningWarning(data.runningWarning);
    setServerView("palworld-settings");
  };

  const openMinecraftProps = async () => {
    if (!selectedId) return;
    const data = await invoke<{ raw: string; exists: boolean }>(
      "read_server_properties",
      { id: selectedId },
    );
    setMcPropsRaw(data.raw);
    setServerView("minecraft-props");
  };

  const applyMcRconFromDisk = (r: {
    enabled: boolean;
    port: number;
    passwordConfigured: boolean;
  }) => {
    setMcRconEnabled(r.enabled);
    setMcRconPort(r.port);
    setMcRconPasswordSet(r.passwordConfigured);
    setMcRconPassword("");
  };

  const syncPalworldSettingsFromDisk = async (id: string) => {
    const cfg = await invoke<InstanceConfig>("sync_palworld_from_ini", { id });
    setConfig(mergeInstanceConfig(cfg));
  };

  const syncMinecraftSettingsFromDisk = async (id: string) => {
    const r = await invoke<{
      enabled: boolean;
      port: number;
      passwordConfigured: boolean;
    }>("sync_minecraft_rcon_from_properties", { id });
    applyMcRconFromDisk(r);
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
            {steamcmdNeeded && (
              <section className="panel steamcmd-banner" style={{ marginBottom: "1rem" }}>
                <strong>{t("steamcmd.needTitle")}</strong>
                <p className="muted" style={{ marginBottom: "0.75rem" }}>
                  {t("steamcmd.needBody")}
                </p>
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={busy}
                  onClick={() =>
                    void run(async () => {
                      await invoke("ensure_steamcmd_cmd");
                      setSteamcmdNeeded(false);
                    })
                  }
                >
                  {t("steamcmd.fetch")}
                </button>
              </section>
            )}
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
                        className="table-row-selectable"
                        onClick={() => setSelectedId(row.id)}
                        style={{
                          background:
                            selectedId === row.id ? "var(--row-hover)" : undefined,
                        }}
                      >
                        <td>
                          {row.displayName}
                          {row.updateAvailable ? (
                            <span className="badge badge-warning" style={{ marginLeft: 8 }}>
                              {t("servers.updateAvailable")}
                            </span>
                          ) : null}
                        </td>
                        <td>
                          <span className="badge">{providerLabel(row)}</span>
                        </td>
                        <td>{statusLabel(row.status)}</td>
                        <td>{row.pid ?? "—"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>

            {selected && (config || detailLoading) && (
              <section className="panel">
                {serverView !== "overview" && (
                  <div className="subview-back">
                    <button
                      type="button"
                      className="btn"
                      onClick={() => setServerView("overview")}
                    >
                      {t("servers.back")}
                    </button>
                  </div>
                )}

                {serverView === "palworld-settings" && config && (
                  <>
                    <h1>{t("servers.gameSettings")}</h1>
                    {palRunningWarning && (
                      <div className="alert">{t("ops.runningEditWarning")}</div>
                    )}
                    <label className="field">
                      <span>{t("ops.palSettingsRaw")}</span>
                      <textarea
                        rows={18}
                        value={palSettingsRaw}
                        onChange={(e) => setPalSettingsRaw(e.target.value)}
                        style={{ maxWidth: "100%", width: "100%" }}
                      />
                    </label>
                    <div className="toolbar">
                      <button
                        type="button"
                        className="btn"
                        disabled={busy}
                        onClick={() =>
                          void run(async () => {
                            const data = await invoke<{
                              raw: string;
                              runningWarning: boolean;
                            }>("read_palworld_settings", { id: selected.id });
                            setPalSettingsRaw(data.raw);
                            setPalRunningWarning(data.runningWarning);
                            await syncPalworldSettingsFromDisk(selected.id);
                          })
                        }
                      >
                        {t("ops.syncFromDisk")}
                      </button>
                      <button
                        type="button"
                        className="btn btn-primary"
                        disabled={busy}
                        onClick={() =>
                          void run(async () => {
                            await invoke("write_palworld_settings", {
                              id: selected.id,
                              raw: palSettingsRaw,
                            });
                            await syncPalworldSettingsFromDisk(selected.id);
                            setServerView("overview");
                          })
                        }
                      >
                        {t("ops.saveConfig")}
                      </button>
                    </div>
                  </>
                )}

                {serverView === "minecraft-props" && config && (
                  <>
                    <h1>{t("servers.gameSettings")}</h1>
                    {selected.status === "running" && (
                      <div className="alert">{t("ops.runningEditWarning")}</div>
                    )}
                    {!mcPropsRaw && (
                      <p className="muted">{t("ops.mcPropsMissing")}</p>
                    )}
                    <label className="field">
                      <span>{t("ops.mcPropsRaw")}</span>
                      <textarea
                        rows={18}
                        value={mcPropsRaw}
                        onChange={(e) => setMcPropsRaw(e.target.value)}
                        style={{ maxWidth: "100%", width: "100%" }}
                      />
                    </label>
                    <div className="toolbar">
                      <button
                        type="button"
                        className="btn"
                        disabled={busy}
                        onClick={() =>
                          void run(async () => {
                            const data = await invoke<{ raw: string }>(
                              "read_server_properties",
                              { id: selected.id },
                            );
                            setMcPropsRaw(data.raw);
                            await syncMinecraftSettingsFromDisk(selected.id);
                          })
                        }
                      >
                        {t("ops.syncFromDisk")}
                      </button>
                      <button
                        type="button"
                        className="btn btn-primary"
                        disabled={busy}
                        onClick={() =>
                          void run(async () => {
                            await invoke("write_server_properties", {
                              id: selected.id,
                              raw: mcPropsRaw,
                            });
                            await syncMinecraftSettingsFromDisk(selected.id);
                            setServerView("overview");
                          })
                        }
                      >
                        {t("ops.saveConfig")}
                      </button>
                    </div>
                  </>
                )}

                {serverView === "overview" && (
                  <>
                <h1>{selected.displayName}</h1>
                <p className="muted path">{selected.path}</p>

                <div className="overview-header">
                  <div className="server-banner">
                    {bannerSrc ? (
                      <img src={bannerSrc} alt="" />
                    ) : null}
                  </div>
                  <div>
                    <p className="muted" style={{ marginTop: 0 }}>
                      <span className="badge">{providerLabel(selected)}</span>{" "}
                      {statusLabel(selected.status)}
                      {selected.pid ? ` · PID ${selected.pid}` : ""}
                    </p>
                    {dashMetrics && (
                      <p className="muted">
                        {t("ops.playersShort")}:{" "}
                        {String(dashMetrics.currentplayernum ?? "—")}/
                        {String(dashMetrics.maxplayernum ?? "—")} · FPS{" "}
                        {String(dashMetrics.serverfps ?? "—")}
                        {selected.updateAvailable
                          ? ` · ${t("servers.updateAvailable")}`
                          : ""}
                      </p>
                    )}
                  </div>
                </div>

                <h2 style={{ fontSize: "1.05rem" }}>{t("ops.hostResources")}</h2>
                <div className="resource-grid">
                  <div className="resource-card">
                    <div className="label">{t("ops.cpu")}</div>
                    <div className="value">
                      {hostResources ? `${hostResources.cpuPercent.toFixed(0)}%` : "—"}
                    </div>
                  </div>
                  <div className="resource-card">
                    <div className="label">{t("ops.memory")}</div>
                    <div className="value">
                      {hostResources
                        ? `${formatBytes(hostResources.memoryUsed)} / ${formatBytes(hostResources.memoryTotal)}`
                        : "—"}
                    </div>
                  </div>
                  <div className="resource-card">
                    <div className="label">{t("ops.disk")}</div>
                    <div className="value">
                      {hostResources
                        ? `${formatBytes(hostResources.diskUsed)} / ${formatBytes(hostResources.diskTotal)}`
                        : "—"}
                    </div>
                  </div>
                </div>

                {detailLoading && (
                  <p className="detail-loading">{t("common.loading")}</p>
                )}

                {!selected.launchable && (
                  <div className="unsupported-msg">{t("servers.unsupportedFolder")}</div>
                )}

                {selected.launchable && config && (
                  <>
                <div className="toolbar">
                  <button
                    type="button"
                    className="btn btn-primary"
                    disabled={busy}
                    onClick={() => void run(async () => invoke("start_server", { id: selected.id }))}
                  >
                    {t("ops.start")}
                  </button>
                  <button
                    type="button"
                    className="btn btn-danger"
                    disabled={busy}
                    onClick={() => void run(async () => invoke("stop_server", { id: selected.id }))}
                  >
                    {t("ops.stop")}
                  </button>
                  <button
                    type="button"
                    className="btn btn-danger"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => invoke("restart_server", { id: selected.id }))
                    }
                  >
                    {t("ops.restart")}
                  </button>
                  {selected.providerId === "palworld" && (
                    <>
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
                    </>
                  )}
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
                  {selected.providerId === "palworld" && (
                    <button type="button" className="btn" onClick={() => void refreshRemoteOps()}>
                      {t("ops.refreshRest")}
                    </button>
                  )}
                  {selected.providerId === "minecraft" && (
                    <button type="button" className="btn" onClick={() => void refreshRemoteOps()}>
                      {t("ops.refreshRcon")}
                    </button>
                  )}
                  {selected.providerId === "palworld" && (
                    <button type="button" className="btn" onClick={() => void openPalworldSettings()}>
                      {t("servers.gameSettings")}
                    </button>
                  )}
                  {selected.providerId === "minecraft" && (
                    <button type="button" className="btn" onClick={() => void openMinecraftProps()}>
                      {t("servers.gameSettings")}
                    </button>
                  )}
                </div>

                {selected.providerId === "minecraft" && (
                  <>
                    <div className="field">
                      <span>{t("servers.serverType")}</span>
                      <select
                        value={config.minecraft.serverType}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            minecraft: {
                              ...config.minecraft,
                              serverType: e.target.value,
                            },
                          })
                        }
                      >
                        <option value="unknown">{t("servers.pickServerType")}</option>
                        {MC_TYPES.map((tp) => (
                          <option key={tp} value={tp}>
                            {t(`servers.minecraftType.${tp}`)}
                          </option>
                        ))}
                      </select>
                      {config.minecraft.serverType === "unknown" && (
                        <span className="muted">{t("servers.serverTypeHint")}</span>
                      )}
                    </div>
                    <div className="field">
                      <span>{t("ops.mcJar")}</span>
                      <input
                        className="btn"
                        value={config.minecraft.jarFile}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            minecraft: { ...config.minecraft, jarFile: e.target.value },
                          })
                        }
                        placeholder="paper.jar"
                      />
                    </div>
                    <div className="field">
                      <span>{t("ops.mcJvmArgs")}</span>
                      <input
                        className="btn"
                        value={config.minecraft.jvmArgs}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            minecraft: { ...config.minecraft, jvmArgs: e.target.value },
                          })
                        }
                      />
                    </div>
                    <div className="field">
                      <span>{t("ops.mcServerArgs")}</span>
                      <input
                        className="btn"
                        value={config.minecraft.serverArgs}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            minecraft: { ...config.minecraft, serverArgs: e.target.value },
                          })
                        }
                      />
                    </div>
                    {selected.status === "running" && (
                      <div className="alert">{t("ops.runningEditWarning")}</div>
                    )}
                    <p className="muted">{t("ops.syncFromDiskHintMinecraft")}</p>
                    <button
                      type="button"
                      className="btn"
                      disabled={busy}
                      onClick={() =>
                        void run(async () => {
                          await syncMinecraftSettingsFromDisk(selected.id);
                        })
                      }
                    >
                      {t("ops.syncFromDisk")}
                    </button>
                    <label className="field">
                      <span>
                        <input
                          type="checkbox"
                          checked={mcRconEnabled}
                          onChange={(e) => setMcRconEnabled(e.target.checked)}
                        />{" "}
                        {t("ops.rconEnabled")}
                      </span>
                    </label>
                    <div className="field">
                      <span>{t("ops.rconPort")}</span>
                      <input
                        className="btn"
                        type="number"
                        min={1}
                        max={65535}
                        value={mcRconPort}
                        onChange={(e) =>
                          setMcRconPort(Number(e.target.value) || 25575)
                        }
                      />
                    </div>
                    <div className="field">
                      <span>
                        {t("ops.rconPassword")}
                        {mcRconPasswordSet ? ` (${t("ops.secretSaved")})` : ""}
                      </span>
                      <input
                        className="btn"
                        type="password"
                        autoComplete="new-password"
                        placeholder={
                          mcRconPasswordSet ? t("ops.secretPlaceholder") : ""
                        }
                        value={mcRconPassword}
                        onChange={(e) => setMcRconPassword(e.target.value)}
                      />
                    </div>
                    <button
                      type="button"
                      className="btn"
                      disabled={busy}
                      onClick={() =>
                        void run(async () => {
                          await invoke("write_minecraft_rcon_settings", {
                            id: selected.id,
                            settings: {
                              enabled: mcRconEnabled,
                              port: mcRconPort,
                              password: mcRconPassword,
                            },
                          });
                          setMcRconPassword("");
                          if (mcRconPassword) {
                            setMcRconPasswordSet(true);
                          }
                        })
                      }
                    >
                      {t("ops.saveRconSettings")}
                    </button>

                    <h2 style={{ marginTop: "1.5rem", fontSize: "1.05rem" }}>
                      {t("ops.rcon")}
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
                            await invoke("minecraft_rcon_announce", {
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
                            await invoke("minecraft_rcon_save", { id: selected.id });
                          })
                        }
                      >
                        Save
                      </button>
                    </div>
                    <div className="toolbar">
                      <input
                        className="btn"
                        style={{ flex: 1 }}
                        value={mcConsoleCmd}
                        placeholder={t("ops.consoleCommand")}
                        onChange={(e) => setMcConsoleCmd(e.target.value)}
                      />
                      <button
                        type="button"
                        className="btn"
                        disabled={!mcConsoleCmd.trim()}
                        onClick={() =>
                          void run(async () => {
                            const out = await invoke<string>("minecraft_rcon_command", {
                              id: selected.id,
                              command: mcConsoleCmd.trim(),
                            });
                            setInfo({ commandOutput: out });
                            setMcConsoleCmd("");
                          })
                        }
                      >
                        {t("ops.runCommand")}
                      </button>
                    </div>
                    <div className="toolbar">
                      <input
                        className="btn"
                        style={{ flex: 1 }}
                        value={unbanId}
                        placeholder={t("ops.unbanPlayer")}
                        onChange={(e) => setUnbanId(e.target.value)}
                      />
                      <button
                        type="button"
                        className="btn"
                        disabled={!unbanId.trim()}
                        onClick={() =>
                          void run(async () => {
                            await invoke("minecraft_rcon_unban", {
                              id: selected.id,
                              player: unbanId.trim(),
                            });
                            setUnbanId("");
                          })
                        }
                      >
                        Unban
                      </button>
                    </div>
                    <table className="table">
                      <thead>
                        <tr>
                          <th>Player</th>
                          <th />
                        </tr>
                      </thead>
                      <tbody>
                        {players.map((p, idx) => {
                          const name = String(p.name ?? p.playerName ?? "?");
                          return (
                            <tr key={`${name}-${idx}`}>
                              <td>{name}</td>
                              <td>
                                <button
                                  type="button"
                                  className="btn"
                                  onClick={() =>
                                    void run(async () => {
                                      await invoke("minecraft_rcon_kick", {
                                        id: selected.id,
                                        player: name,
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
                                      await invoke("minecraft_rcon_ban", {
                                        id: selected.id,
                                        player: name,
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
                  </>
                )}

                {selected.providerId === "palworld" && (
                  <>
                {selected.status === "running" && (
                  <div className="alert">{t("ops.runningEditWarning")}</div>
                )}
                <p className="muted">{t("ops.syncFromDiskHintPalworld")}</p>
                <button
                  type="button"
                  className="btn"
                  disabled={busy}
                  onClick={() =>
                    void run(async () => {
                      await syncPalworldSettingsFromDisk(selected.id);
                    })
                  }
                >
                  {t("ops.syncFromDisk")}
                </button>
                <label className="field">
                  <span>
                    <input
                      type="checkbox"
                      checked={config.restApiEnabled}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          restApiEnabled: e.target.checked,
                        })
                      }
                    />{" "}
                    {t("ops.restApiEnabled")}
                  </span>
                </label>
                <div className="field">
                  <span>{t("ops.restApiPort")}</span>
                  <input
                    className="btn"
                    type="number"
                    min={1}
                    max={65535}
                    value={config.restApiPort}
                    onChange={(e) => {
                      const port = Number(e.target.value) || 8212;
                      setConfig({
                        ...config,
                        restApiPort: port,
                        restBaseUrl: `http://127.0.0.1:${port}/v1/api`,
                      });
                    }}
                  />
                </div>
                <div className="field">
                  <span>{t("ops.restUrl")}</span>
                  <input className="btn" value={config.restBaseUrl} readOnly />
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

                  </>
                )}

                <div className="toolbar">
                  <button
                    type="button"
                    className="btn btn-primary"
                    disabled={busy || !config}
                    onClick={() =>
                      config &&
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
                </div>

                {selected.providerId === "palworld" && (
                  <>
                <h2 style={{ marginTop: "1.5rem", fontSize: "1.05rem" }}>
                  {t("ops.saveParser")}
                </h2>
                {saveLoading && (
                  <p className="detail-loading">{t("ops.saveLoading")}</p>
                )}
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
                  </>
                )}

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
                  </>
                )}
                  </>
                )}
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
            <div className="field">
              <span>{t("settings.appRoot")}</span>
              <code className="path">{appRoot || "—"}</code>
            </div>
            <p className="muted">{t("settings.unsignedNote")}</p>
          </section>
        )}
      </main>

      {closePrompt && (
        <div className="modal-overlay" role="dialog" aria-modal="true">
          <div className="modal-panel">
            <h2>{t("app.closeConfirmTitle")}</h2>
            <p>
              {closeShuttingDown
                ? t("app.closeConfirmShuttingDown")
                : t("app.closeConfirmBody", { names: closePrompt.join(", ") })}
            </p>
            <div className="modal-actions">
              <button
                type="button"
                className="btn"
                disabled={closeShuttingDown}
                onClick={() => setClosePrompt(null)}
              >
                {t("app.closeConfirmStay")}
              </button>
              <button
                type="button"
                className="btn btn-danger"
                disabled={closeShuttingDown}
                onClick={() => void confirmAppExit()}
              >
                {t("app.closeConfirmExit")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
