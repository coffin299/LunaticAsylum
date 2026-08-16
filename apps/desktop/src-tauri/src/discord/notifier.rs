use crate::config::DiscordNotifyConfig;
use serenity::all::{ChannelId, Colour, CreateEmbed, Http, Timestamp};
use serde_json::Value;

pub fn player_key(player: &Value) -> String {
    if let Some(v) = player.get("userId").or_else(|| player.get("userid")) {
        return v.to_string().trim_matches('"').to_string();
    }
    if let Some(v) = player.get("playerId") {
        return v.to_string().trim_matches('"').to_string();
    }
    player
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

pub fn player_display_name(player: &Value) -> String {
    player
        .get("name")
        .and_then(|v| v.as_str())
        .or_else(|| player.get("accountName").and_then(|v| v.as_str()))
        .unwrap_or("Unknown")
        .to_string()
}

pub struct Notifier {
    http: ArcHttp,
    channel_id: ChannelId,
    topic_template: String,
    notify: DiscordNotifyConfig,
    last_topic: Option<String>,
    topic_dirty: bool,
}

/// Http を共有するための薄いラップ
#[derive(Clone)]
pub struct ArcHttp(pub std::sync::Arc<Http>);

impl Notifier {
    pub fn new(
        http: std::sync::Arc<Http>,
        channel_id: ChannelId,
        topic_template: String,
        notify: DiscordNotifyConfig,
    ) -> Self {
        Self {
            http: ArcHttp(http),
            channel_id,
            topic_template,
            notify,
            last_topic: None,
            topic_dirty: false,
        }
    }

    async fn send_embed(&self, embed: CreateEmbed) {
        let builder = serenity::all::CreateMessage::new().embed(embed);
        let _ = self.channel_id.send_message(&self.http.0, builder).await;
    }

    pub async fn notify_startup(
        &self,
        info: Option<&Value>,
        players: &[Value],
        reachable: bool,
    ) {
        if !self.notify.rest_status {
            return;
        }
        let mut embed = CreateEmbed::new()
            .title("LunaticAsylum Discord 起動")
            .colour(if reachable {
                Colour::DARK_GREEN
            } else {
                Colour::ORANGE
            })
            .timestamp(Timestamp::now())
            .field(
                "REST API",
                if reachable { "接続成功" } else { "未到達" },
                true,
            )
            .field("オンライン", players.len().to_string(), true);
        if let Some(info) = info {
            embed = embed
                .field(
                    "サーバー名",
                    info.get("servername")
                        .and_then(|v| v.as_str())
                        .unwrap_or("不明"),
                    true,
                )
                .field(
                    "バージョン",
                    info.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("不明"),
                    true,
                );
        }
        if !players.is_empty() {
            let mut names: Vec<String> = players
                .iter()
                .take(20)
                .map(player_display_name)
                .collect();
            if players.len() > 20 {
                names.push(format!("他 {} 人", players.len() - 20));
            }
            embed = embed.field("プレイヤー", names.join(", "), false);
        }
        self.send_embed(embed).await;
    }

    pub async fn notify_join(&self, player: &Value, current: i64, maximum: i64) {
        if !self.notify.join_leave {
            return;
        }
        let mut embed = CreateEmbed::new()
            .title("プレイヤー入室")
            .description(player_display_name(player))
            .colour(Colour::BLUE)
            .timestamp(Timestamp::now())
            .field("人数", format!("{current}/{maximum}"), true);
        if let Some(level) = player.get("level") {
            embed = embed.field("レベル", level.to_string(), true);
        }
        self.send_embed(embed).await;
    }

    pub async fn notify_leave(&self, player: &Value, current: i64, maximum: i64) {
        if !self.notify.join_leave {
            return;
        }
        let mut embed = CreateEmbed::new()
            .title("プレイヤー退室")
            .description(player_display_name(player))
            .colour(Colour::DARK_GREY)
            .timestamp(Timestamp::now())
            .field("人数", format!("{current}/{maximum}"), true);
        if let Some(level) = player.get("level") {
            embed = embed.field("レベル", level.to_string(), true);
        }
        self.send_embed(embed).await;
    }

    pub async fn notify_unreachable(&self, detail: &str) {
        if !self.notify.rest_status {
            return;
        }
        let embed = CreateEmbed::new()
            .title("サーバー未到達")
            .description(detail)
            .colour(Colour::RED)
            .timestamp(Timestamp::now());
        self.send_embed(embed).await;
    }

    pub async fn notify_recovered(&self, player_count: usize) {
        if !self.notify.rest_status {
            return;
        }
        let embed = CreateEmbed::new()
            .title("サーバー復旧")
            .description(format!("REST API に再接続しました。オンライン: {player_count}"))
            .colour(Colour::DARK_GREEN)
            .timestamp(Timestamp::now());
        self.send_embed(embed).await;
    }

    pub async fn update_topic(&mut self, current: i64, maximum: i64, force: bool) {
        if !self.notify.topic {
            return;
        }
        let topic = self
            .topic_template
            .replace("{current}", &current.to_string())
            .replace("{max}", &maximum.to_string());
        if !force && !self.topic_dirty && self.last_topic.as_deref() == Some(topic.as_str()) {
            return;
        }
        let edit = serenity::all::EditChannel::new().topic(&topic);
        match self.channel_id.edit(&self.http.0, edit).await {
            Ok(_) => {
                self.last_topic = Some(topic);
                self.topic_dirty = false;
            }
            Err(_) => {
                self.topic_dirty = true;
            }
        }
    }
}
