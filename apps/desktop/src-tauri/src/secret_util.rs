//! 秘密の取り扱い（ログ赤化・Debug で出さない）

use std::fmt;

/// ログやエラーに載せない秘密文字列。
#[derive(Clone, Default)]
#[allow(dead_code)] // 今後ログ/型境界で使用
pub struct SecretString(String);

impl SecretString {
    #[allow(dead_code)]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    #[allow(dead_code)]
    pub fn expose(&self) -> &str {
        &self.0
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// 既知パターンを雑にマスク（ログ用）。完全ではないが平文流出を減らす。
pub fn redact_text(input: &str) -> String {
    let mut out = input.to_string();
    for needle in [
        "Authorization:",
        "Bot ",
        "password",
        "Password",
        "token",
        "Token",
    ] {
        if out.contains(needle) {
            out = out.replace(needle, "[REDACTED_HINT]");
        }
    }
    out
}
