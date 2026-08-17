//! Discord Bot（スラッシュ + ポーラー）

use super::notifier::{player_display_name, Notifier};
use super::poller::PlayerPoller;
use crate::config::DiscordConfig;
use crate::palworld_rest::PalworldRestClient;
use crate::state::AppState;
use serenity::all::{
    ButtonStyle, Colour, CommandDataOptionValue, CommandOptionType, ComponentInteractionDataKind,
    CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage, GuildId, Interaction,
    Ready, Timestamp,
};
use serenity::async_trait;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

enum PendingAction {
    Shutdown {
        owner_id: u64,
        instance_id: String,
        waittime: i64,
        message: String,
    },
    Stop {
        owner_id: u64,
        instance_id: String,
    },
}

struct BotData {
    instance_id: String,
    config: DiscordConfig,
    rest: Arc<PalworldRestClient>,
    app_state: Arc<std::sync::Mutex<AppState>>,
    admin_ids: Vec<u64>,
    notifier: Option<Arc<Mutex<Notifier>>>,
    poller_started: bool,
    pending: std::collections::HashMap<String, PendingAction>,
}

struct Handler {
    data: Arc<Mutex<BotData>>,
}

fn truncate(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        return s.to_string();
    }
    let t: String = s.chars().take(limit.saturating_sub(3)).collect();
    format!("{t}...")
}

fn is_admin(user_id: u64, admins: &[u64]) -> bool {
    admins.contains(&user_id)
}

async fn deny_admin(ctx: &Context, interaction: &serenity::all::CommandInteraction, admins: &[u64]) -> bool {
    if is_admin(interaction.user.id.get(), admins) {
        return false;
    }
    let _ = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("このコマンドは管理者のみ実行できます。")
                    .ephemeral(true),
            ),
        )
        .await;
    true
}

fn option_str(interaction: &serenity::all::CommandInteraction, name: &str) -> Option<String> {
    interaction
        .data
        .options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
}

fn option_i64(interaction: &serenity::all::CommandInteraction, name: &str, default: i64) -> i64 {
    interaction
        .data
        .options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(default)
}

async fn defer_ephemeral(ctx: &Context, interaction: &serenity::all::CommandInteraction) {
    let _ = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;
}

async fn followup(ctx: &Context, interaction: &serenity::all::CommandInteraction, content: impl Into<String>) {
    let _ = interaction
        .create_followup(
            &ctx.http,
            serenity::all::CreateInteractionResponseFollowup::new()
                .content(content)
                .ephemeral(true),
        )
        .await;
}

async fn followup_embed(
    ctx: &Context,
    interaction: &serenity::all::CommandInteraction,
    embed: CreateEmbed,
) {
    let _ = interaction
        .create_followup(
            &ctx.http,
            serenity::all::CreateInteractionResponseFollowup::new()
                .embed(embed)
                .ephemeral(true),
        )
        .await;
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Discord ready: {}", ready.user.name);
        let mut data = self.data.lock().await;
        let guild_id: u64 = match data.config.guild_id.parse() {
            Ok(v) => v,
            Err(_) => return,
        };
        let channel_id: u64 = match data.config.channel_id.parse() {
            Ok(v) => v,
            Err(_) => return,
        };
        let guild = GuildId::new(guild_id);
        let commands = build_commands();
        let _ = guild.set_commands(&ctx.http, commands).await;

        let notifier = Arc::new(Mutex::new(Notifier::new(
            Arc::clone(&ctx.http),
            serenity::all::ChannelId::new(channel_id),
            data.config.topic_template.clone(),
            data.config.notify.clone(),
        )));
        data.notifier = Some(Arc::clone(&notifier));

        if !data.poller_started {
            data.poller_started = true;
            let rest = Arc::clone(&data.rest);
            let interval = data.config.poll_interval_seconds;
            tokio::spawn(async move {
                let poller = PlayerPoller::new(rest, notifier, interval);
                let (_keep_alive, stop_rx) = oneshot::channel::<()>();
                poller.run(stop_rx).await;
            });
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(cmd) => {
                self.handle_command(ctx, cmd).await;
            }
            Interaction::Component(comp) => {
                self.handle_component(ctx, comp).await;
            }
            _ => {}
        }
    }
}

impl Handler {
    async fn handle_command(&self, ctx: Context, cmd: serenity::all::CommandInteraction) {
        let (name, rest, admins, instance_id, app_state) = {
            let data = self.data.lock().await;
            (
                cmd.data.name.clone(),
                Arc::clone(&data.rest),
                data.admin_ids.clone(),
                data.instance_id.clone(),
                Arc::clone(&data.app_state),
            )
        };

        match name.as_str() {
            "help" => {
                let embed = CreateEmbed::new()
                    .title("LunaticAsylum Discord Help")
                    .description(
                        "Palworld サーバー向け Discord 連携です。\nREST API 経由で入退出通知・人数表示・サーバー管理ができます。",
                    )
                    .colour(Colour::BLURPLE)
                    .timestamp(Timestamp::now())
                    .field(
                        "自動通知",
                        "・プレイヤー入室 / 退室\n・REST API 未到達 / 復旧\n・起動時ステータス\n・チャンネルトピックの人数表示",
                        false,
                    )
                    .field(
                        "一般コマンド",
                        "`/help` `/players` `/info` `/metrics` `/settings`",
                        false,
                    )
                    .field(
                        "管理コマンド（admin_ids のみ）",
                        "`/kick` `/ban` `/unban` `/announce` `/save` `/shutdown` `/stop`\n`/shutdown`・`/stop` は Confirm 必須",
                        false,
                    );
                let _ = cmd
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(embed)
                                .ephemeral(true),
                        ),
                    )
                    .await;
            }
            "players" => {
                defer_ephemeral(&ctx, &cmd).await;
                match rest.get_players() {
                    Ok(players) => {
                        let mut embed = CreateEmbed::new()
                            .title("オンラインプレイヤー")
                            .description(format!("{} 人", players.len()))
                            .colour(Colour::BLURPLE)
                            .timestamp(Timestamp::now());
                        if players.is_empty() {
                            embed = embed.field("一覧", "誰もオンラインではありません。", false);
                        } else {
                            let lines: Vec<String> = players
                                .iter()
                                .map(|p| {
                                    let name = player_display_name(p);
                                    let level = p.get("level").map(|v| v.to_string()).unwrap_or_else(|| "?".into());
                                    let uid = p
                                        .get("userId")
                                        .or_else(|| p.get("userid"))
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "?".into());
                                    format!("**{name}** (Lv {level}) — `{uid}`")
                                })
                                .collect();
                            embed = embed.field("一覧", truncate(&lines.join("\n"), 1000), false);
                        }
                        followup_embed(&ctx, &cmd, embed).await;
                    }
                    Err(e) => followup(&ctx, &cmd, format!("取得に失敗しました: {e}")).await,
                }
            }
            "info" => {
                defer_ephemeral(&ctx, &cmd).await;
                match rest.get_info() {
                    Ok(data) => {
                        let embed = CreateEmbed::new()
                            .title("サーバー情報")
                            .colour(Colour::DARK_GREEN)
                            .timestamp(Timestamp::now())
                            .field(
                                "名前",
                                data.get("servername").and_then(|v| v.as_str()).unwrap_or("不明"),
                                true,
                            )
                            .field(
                                "バージョン",
                                data.get("version").and_then(|v| v.as_str()).unwrap_or("不明"),
                                true,
                            )
                            .field(
                                "説明",
                                truncate(
                                    data.get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("(なし)"),
                                    500,
                                ),
                                false,
                            )
                            .field(
                                "World GUID",
                                format!(
                                    "`{}`",
                                    data.get("worldguid")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("不明")
                                ),
                                false,
                            );
                        followup_embed(&ctx, &cmd, embed).await;
                    }
                    Err(e) => followup(&ctx, &cmd, format!("取得に失敗しました: {e}")).await,
                }
            }
            "metrics" => {
                defer_ephemeral(&ctx, &cmd).await;
                match rest.get_metrics() {
                    Ok(data) => {
                        let embed = CreateEmbed::new()
                            .title("サーバーメトリクス")
                            .colour(Colour::TEAL)
                            .timestamp(Timestamp::now())
                            .field(
                                "FPS",
                                data.get("serverfps")
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "?".into()),
                                true,
                            )
                            .field(
                                "プレイヤー",
                                format!(
                                    "{}/{}",
                                    data.get("currentplayernum")
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "?".into()),
                                    data.get("maxplayernum")
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "?".into())
                                ),
                                true,
                            )
                            .field(
                                "Frame time (ms)",
                                data.get("serverframetime")
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "?".into()),
                                true,
                            )
                            .field(
                                "Uptime (秒)",
                                data.get("uptime")
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "?".into()),
                                true,
                            )
                            .field(
                                "拠点数",
                                data.get("basecampnum")
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "?".into()),
                                true,
                            )
                            .field(
                                "ゲーム内日数",
                                data.get("days")
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "?".into()),
                                true,
                            );
                        followup_embed(&ctx, &cmd, embed).await;
                    }
                    Err(e) => followup(&ctx, &cmd, format!("取得に失敗しました: {e}")).await,
                }
            }
            "settings" => {
                defer_ephemeral(&ctx, &cmd).await;
                match rest.get_settings() {
                    Ok(data) => {
                        let pretty = serde_json::to_string_pretty(&data).unwrap_or_default();
                        let embed = CreateEmbed::new()
                            .title("サーバー設定")
                            .description(format!("```json\n{}\n```", truncate(&pretty, 3500)))
                            .colour(Colour::DARK_GOLD)
                            .timestamp(Timestamp::now());
                        followup_embed(&ctx, &cmd, embed).await;
                    }
                    Err(e) => followup(&ctx, &cmd, format!("取得に失敗しました: {e}")).await,
                }
            }
            "kick" => {
                if deny_admin(&ctx, &cmd, &admins).await {
                    return;
                }
                defer_ephemeral(&ctx, &cmd).await;
                let userid = option_str(&cmd, "userid").unwrap_or_default();
                let message =
                    option_str(&cmd, "message").unwrap_or_else(|| "Kicked by admin".into());
                match rest.kick(&userid, &message) {
                    Ok(()) => {
                        followup(
                            &ctx,
                            &cmd,
                            format!("`{userid}` をキックしました。理由: {message}"),
                        )
                        .await
                    }
                    Err(e) => followup(&ctx, &cmd, format!("失敗しました: {e}")).await,
                }
            }
            "ban" => {
                if deny_admin(&ctx, &cmd, &admins).await {
                    return;
                }
                defer_ephemeral(&ctx, &cmd).await;
                let userid = option_str(&cmd, "userid").unwrap_or_default();
                let message =
                    option_str(&cmd, "message").unwrap_or_else(|| "Banned by admin".into());
                match rest.ban(&userid, &message) {
                    Ok(()) => {
                        followup(
                            &ctx,
                            &cmd,
                            format!("`{userid}` を BAN しました。理由: {message}"),
                        )
                        .await
                    }
                    Err(e) => followup(&ctx, &cmd, format!("失敗しました: {e}")).await,
                }
            }
            "unban" => {
                if deny_admin(&ctx, &cmd, &admins).await {
                    return;
                }
                defer_ephemeral(&ctx, &cmd).await;
                let userid = option_str(&cmd, "userid").unwrap_or_default();
                match rest.unban(&userid) {
                    Ok(()) => {
                        followup(&ctx, &cmd, format!("`{userid}` の BAN を解除しました。")).await
                    }
                    Err(e) => followup(&ctx, &cmd, format!("失敗しました: {e}")).await,
                }
            }
            "announce" => {
                if deny_admin(&ctx, &cmd, &admins).await {
                    return;
                }
                defer_ephemeral(&ctx, &cmd).await;
                let message = option_str(&cmd, "message").unwrap_or_default();
                match rest.announce(&message) {
                    Ok(()) => followup(&ctx, &cmd, format!("アナウンスしました: {message}")).await,
                    Err(e) => followup(&ctx, &cmd, format!("失敗しました: {e}")).await,
                }
            }
            "save" => {
                if deny_admin(&ctx, &cmd, &admins).await {
                    return;
                }
                defer_ephemeral(&ctx, &cmd).await;
                match rest.save() {
                    Ok(()) => followup(&ctx, &cmd, "ワールドを保存しました。").await,
                    Err(e) => followup(&ctx, &cmd, format!("失敗しました: {e}")).await,
                }
            }
            "shutdown" => {
                if deny_admin(&ctx, &cmd, &admins).await {
                    return;
                }
                let waittime = option_i64(&cmd, "waittime", 60);
                let message = option_str(&cmd, "message")
                    .unwrap_or_else(|| "Server is shutting down".into());
                let confirm_id = format!("c{}", cmd.id.get());
                {
                    let mut data = self.data.lock().await;
                    data.pending.insert(
                        confirm_id.clone(),
                        PendingAction::Shutdown {
                            owner_id: cmd.user.id.get(),
                            instance_id: instance_id.clone(),
                            waittime,
                            message: message.clone(),
                        },
                    );
                }
                let embed = CreateEmbed::new()
                    .title("シャットダウン確認")
                    .description(format!(
                        "**{waittime} 秒後**にサーバーをシャットダウンします。\nメッセージ: {message}\n\n実行する場合は Confirm を押してください。"
                    ))
                    .colour(Colour::ORANGE)
                    .timestamp(Timestamp::now());
                let row = CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("asylum:ok:{confirm_id}"))
                        .label("Confirm")
                        .style(ButtonStyle::Danger),
                    CreateButton::new(format!("asylum:no:{confirm_id}"))
                        .label("Cancel")
                        .style(ButtonStyle::Secondary),
                ]);
                let _ = cmd
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(embed)
                                .components(vec![row])
                                .ephemeral(true),
                        ),
                    )
                    .await;
                let _ = app_state;
            }
            "stop" => {
                if deny_admin(&ctx, &cmd, &admins).await {
                    return;
                }
                let confirm_id = format!("c{}", cmd.id.get());
                {
                    let mut data = self.data.lock().await;
                    data.pending.insert(
                        confirm_id.clone(),
                        PendingAction::Stop {
                            owner_id: cmd.user.id.get(),
                            instance_id: instance_id.clone(),
                        },
                    );
                }
                let embed = CreateEmbed::new()
                    .title("強制停止の確認")
                    .description(
                        "サーバーを**即時強制停止**します。セーブされない可能性があります。\n\n実行する場合は Confirm を押してください。",
                    )
                    .colour(Colour::RED)
                    .timestamp(Timestamp::now());
                let row = CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("asylum:ok:{confirm_id}"))
                        .label("Confirm")
                        .style(ButtonStyle::Danger),
                    CreateButton::new(format!("asylum:no:{confirm_id}"))
                        .label("Cancel")
                        .style(ButtonStyle::Secondary),
                ]);
                let _ = cmd
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(embed)
                                .components(vec![row])
                                .ephemeral(true),
                        ),
                    )
                    .await;
            }
            _ => {}
        }
    }

    async fn handle_component(&self, ctx: Context, comp: serenity::all::ComponentInteraction) {
        let id = comp.data.custom_id.clone();
        if !matches!(comp.data.kind, ComponentInteractionDataKind::Button) {
            return;
        }

        if let Some(confirm_id) = id.strip_prefix("asylum:no:") {
            let mut data = self.data.lock().await;
            data.pending.remove(confirm_id);
            let _ = comp
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .content("キャンセルしました。")
                            .embeds(vec![])
                            .components(vec![]),
                    ),
                )
                .await;
            return;
        }

        let Some(confirm_id) = id.strip_prefix("asylum:ok:") else {
            return;
        };

        let (admins, rest, app_state, action) = {
            let mut data = self.data.lock().await;
            (
                data.admin_ids.clone(),
                Arc::clone(&data.rest),
                Arc::clone(&data.app_state),
                data.pending.remove(confirm_id),
            )
        };

        let Some(action) = action else {
            let _ = comp
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("確認の有効期限が切れています。")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        };

        let owner_ok = match &action {
            PendingAction::Shutdown { owner_id, .. } | PendingAction::Stop { owner_id, .. } => {
                *owner_id == comp.user.id.get()
            }
        };
        if !owner_ok {
            let _ = comp
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("この確認はコマンド実行者のみ操作できます。")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
        if !is_admin(comp.user.id.get(), &admins) {
            let _ = comp
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("管理者権限がありません。")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        let _ = comp
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Defer(
                    CreateInteractionResponseMessage::new().ephemeral(true),
                ),
            )
            .await;

        let msg = match action {
            PendingAction::Stop { instance_id, .. } => {
                if let Err(e) = rest.stop() {
                    if let Ok(mut g) = app_state.lock() {
                        let _ = g.stop_intentional(&instance_id);
                    }
                    format!("REST 失敗、プロセス終了を試行: {e}")
                } else {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if let Ok(mut g) = app_state.lock() {
                        match g.stop_intentional(&instance_id) {
                            Ok(()) => "停止しました。".to_string(),
                            Err(e) => format!("REST 停止後のプロセス終了に失敗: {e}"),
                        }
                    } else {
                        "停止を要求しました。".to_string()
                    }
                }
            }
            PendingAction::Shutdown {
                instance_id: _,
                waittime,
                message,
                ..
            } => {
                match rest.shutdown(waittime, &message) {
                    Ok(()) => {
                        format!("シャットダウンを開始しました（{waittime} 秒 / {message}）。")
                    }
                    Err(e) => format!("失敗しました: {e}"),
                }
            }
        };
        let _ = comp
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .content(msg)
                    .ephemeral(true),
            )
            .await;
    }
}

fn build_commands() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("help").description("使い方とコマンド一覧"),
        CreateCommand::new("players").description("オンラインプレイヤー一覧"),
        CreateCommand::new("info").description("サーバー情報"),
        CreateCommand::new("metrics").description("サーバーメトリクス"),
        CreateCommand::new("settings").description("サーバー設定の要約"),
        CreateCommand::new("kick")
            .description("プレイヤーをキック（管理者）")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "userid", "対象の userId")
                    .required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "message",
                "キック理由",
            )),
        CreateCommand::new("ban")
            .description("プレイヤーを BAN（管理者）")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "userid", "対象の userId")
                    .required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "message",
                "BAN 理由",
            )),
        CreateCommand::new("unban")
            .description("BAN 解除（管理者）")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "userid", "対象の userId")
                    .required(true),
            ),
        CreateCommand::new("announce")
            .description("全体アナウンス（管理者）")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "message", "アナウンス文言")
                    .required(true),
            ),
        CreateCommand::new("save").description("ワールド保存（管理者）"),
        CreateCommand::new("shutdown")
            .description("猶予付きシャットダウン（管理者・要確認）")
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "waittime",
                "秒数",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "message",
                "メッセージ",
            )),
        CreateCommand::new("stop").description("即時強制停止（管理者・要確認）"),
    ]
}

pub async fn run_bot(
    instance_id: String,
    config: DiscordConfig,
    rest: PalworldRestClient,
    app_state: Arc<std::sync::Mutex<AppState>>,
    stop: oneshot::Receiver<()>,
) {
    let admin_ids = config
        .admin_ids
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect::<Vec<u64>>();

    let data = Arc::new(Mutex::new(BotData {
        instance_id,
        config: config.clone(),
        rest: Arc::new(rest),
        app_state,
        admin_ids,
        notifier: None,
        poller_started: false,
        pending: std::collections::HashMap::new(),
    }));

    let token = config.token.clone();
    let intents = GatewayIntents::GUILDS;
    let mut client = match Client::builder(&token, intents)
        .event_handler(Handler {
            data: Arc::clone(&data),
        })
        .await
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Discord client build failed: {e}");
            return;
        }
    };

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        let _ = stop.await;
        shard_manager.shutdown_all().await;
    });

    if let Err(e) = client.start().await {
        eprintln!("Discord client error: {e}");
    }
}
