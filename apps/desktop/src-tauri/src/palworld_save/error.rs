//! パースエラーと診断（panic しない）

use std::fmt;

#[derive(Debug, Clone)]
pub struct ParseDiag {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Default, Clone)]
pub struct ParseStats {
    pub skipped_properties: u64,
    pub unsupported_types: u64,
    pub subsection_failures: u64,
    pub diags: Vec<ParseDiag>,
}

impl ParseStats {
    pub fn push(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.diags.push(ParseDiag {
            code: code.into(),
            message: message.into(),
        });
    }
}

#[derive(Debug)]
pub enum ParseError {
    Io(String),
    Format(String),
    Unsupported(String),
    Eof(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(s) | Self::Format(s) | Self::Unsupported(s) | Self::Eof(s) => {
                write!(f, "{s}")
            }
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
