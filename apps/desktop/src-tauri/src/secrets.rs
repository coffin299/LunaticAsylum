//! OS 資格情報ストア（Windows Credential Manager 等）
//! Discord Token / REST パスワードはここに置き、config.json には書かない。

use keyring::Entry;

const SERVICE: &str = "LunaticAsylum";

fn entry(instance_id: &str, kind: &str) -> Result<Entry, String> {
    let user = format!("{instance_id}/{kind}");
    Entry::new(SERVICE, &user).map_err(|e| format!("keyring entry: {e}"))
}

pub fn set_secret(instance_id: &str, kind: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return delete_secret(instance_id, kind);
    }
    entry(instance_id, kind)?
        .set_password(value)
        .map_err(|e| format!("secret store failed: {e}"))
}

pub fn get_secret(instance_id: &str, kind: &str) -> Result<Option<String>, String> {
    match entry(instance_id, kind)?.get_password() {
        Ok(v) if !v.is_empty() => Ok(Some(v)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("secret read failed: {e}")),
    }
}

pub fn delete_secret(instance_id: &str, kind: &str) -> Result<(), String> {
    match entry(instance_id, kind)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("secret delete failed: {e}")),
    }
}

pub const KIND_DISCORD_TOKEN: &str = "discord.token";
pub const KIND_REST_PASSWORD: &str = "rest.password";

pub fn set_discord_token(instance_id: &str, token: &str) -> Result<(), String> {
    set_secret(instance_id, KIND_DISCORD_TOKEN, token)
}

pub fn get_discord_token(instance_id: &str) -> Result<Option<String>, String> {
    get_secret(instance_id, KIND_DISCORD_TOKEN)
}

pub fn set_rest_password(instance_id: &str, password: &str) -> Result<(), String> {
    set_secret(instance_id, KIND_REST_PASSWORD, password)
}

pub fn get_rest_password(instance_id: &str) -> Result<Option<String>, String> {
    get_secret(instance_id, KIND_REST_PASSWORD)
}
