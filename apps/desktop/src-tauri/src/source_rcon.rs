//! Source RCON プロトコル（Minecraft Java 等）

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const SERVERDATA_AUTH: i32 = 3;
const SERVERDATA_AUTH_RESPONSE: i32 = 2;
const SERVERDATA_EXECCOMMAND: i32 = 2;
const SERVERDATA_RESPONSE_VALUE: i32 = 0;

const MAX_PACKET_SIZE: usize = 4096;
const MAX_RESPONSE_BYTES: usize = 65_536;

#[derive(Debug)]
pub struct SourceRconError {
    pub message: String,
}

impl std::fmt::Display for SourceRconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct SourceRconClient {
    stream: TcpStream,
    next_id: i32,
    timeout: Duration,
}

impl SourceRconClient {
    pub fn connect(host: &str, port: u16, password: &str, timeout_ms: u64) -> Result<Self, SourceRconError> {
        let addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|_| SourceRconError {
                message: "invalid RCON address".into(),
            })?;
        if !addr.ip().is_loopback() {
            return Err(SourceRconError {
                message: "RCON host must be localhost".into(),
            });
        }
        let timeout = Duration::from_millis(timeout_ms.max(1000));
        let stream = TcpStream::connect_timeout(&addr, timeout).map_err(|e| SourceRconError {
            message: format!("RCON connect failed: {e}"),
        })?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| SourceRconError {
                message: format!("RCON timeout config failed: {e}"),
            })?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| SourceRconError {
                message: format!("RCON timeout config failed: {e}"),
            })?;
        let mut client = Self {
            stream,
            next_id: 1,
            timeout,
        };
        client.auth(password)?;
        Ok(client)
    }

    pub fn command(&mut self, cmd: &str) -> Result<String, SourceRconError> {
        if cmd.len() > 4096 {
            return Err(SourceRconError {
                message: "RCON command too long".into(),
            });
        }
        let id = self.next_id();
        self.send_packet(id, SERVERDATA_EXECCOMMAND, cmd)?;
        self.read_command_response(id)
    }

    fn auth(&mut self, password: &str) -> Result<(), SourceRconError> {
        let id = self.next_id();
        self.send_packet(id, SERVERDATA_AUTH, password)?;
        loop {
            let resp = self.read_packet()?;
            if resp.id == -1 {
                return Err(SourceRconError {
                    message: "RCON authentication failed".into(),
                });
            }
            if resp.id == id && resp.kind == SERVERDATA_AUTH_RESPONSE {
                return Ok(());
            }
        }
    }

    fn next_id(&mut self) -> i32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1;
        }
        id
    }

    fn send_packet(&mut self, id: i32, kind: i32, body: &str) -> Result<(), SourceRconError> {
        let body_bytes = body.as_bytes();
        let size = (4 + 4 + body_bytes.len() + 2) as i32;
        if size as usize > MAX_PACKET_SIZE {
            return Err(SourceRconError {
                message: "RCON packet too large".into(),
            });
        }
        self.stream
            .write_all(&size.to_le_bytes())
            .map_err(|e| SourceRconError {
                message: format!("RCON write failed: {e}"),
            })?;
        self.stream
            .write_all(&id.to_le_bytes())
            .map_err(|e| SourceRconError {
                message: format!("RCON write failed: {e}"),
            })?;
        self.stream
            .write_all(&kind.to_le_bytes())
            .map_err(|e| SourceRconError {
                message: format!("RCON write failed: {e}"),
            })?;
        self.stream
            .write_all(body_bytes)
            .map_err(|e| SourceRconError {
                message: format!("RCON write failed: {e}"),
            })?;
        self.stream.write_all(&[0, 0]).map_err(|e| SourceRconError {
            message: format!("RCON write failed: {e}"),
        })?;
        self.stream.flush().map_err(|e| SourceRconError {
            message: format!("RCON flush failed: {e}"),
        })?;
        Ok(())
    }

    fn read_packet(&mut self) -> Result<RconPacket, SourceRconError> {
        let mut size_buf = [0u8; 4];
        self.stream
            .read_exact(&mut size_buf)
            .map_err(|e| SourceRconError {
                message: format!("RCON read failed: {e}"),
            })?;
        let size = i32::from_le_bytes(size_buf);
        if size <= 0 || size as usize > MAX_PACKET_SIZE {
            return Err(SourceRconError {
                message: "invalid RCON packet size".into(),
            });
        }
        let mut buf = vec![0u8; size as usize];
        self.stream.read_exact(&mut buf).map_err(|e| SourceRconError {
            message: format!("RCON read failed: {e}"),
        })?;
        if buf.len() < 8 {
            return Err(SourceRconError {
                message: "RCON packet truncated".into(),
            });
        }
        let id = i32::from_le_bytes(buf[0..4].try_into().unwrap());
        let kind = i32::from_le_bytes(buf[4..8].try_into().unwrap());
        let body_end = buf.len().saturating_sub(2);
        let body = String::from_utf8_lossy(&buf[8..body_end]).into_owned();
        Ok(RconPacket { id, kind, body })
    }

    fn read_command_response(&mut self, req_id: i32) -> Result<String, SourceRconError> {
        let mut out = String::new();
        loop {
            let packet = self.read_packet()?;
            if packet.kind != SERVERDATA_RESPONSE_VALUE || packet.id != req_id {
                continue;
            }
            if packet.body.is_empty() {
                break;
            }
            if out.len() + packet.body.len() > MAX_RESPONSE_BYTES {
                return Err(SourceRconError {
                    message: "RCON response too large".into(),
                });
            }
            out.push_str(&packet.body);
        }
        Ok(out.trim().to_string())
    }
}

struct RconPacket {
    id: i32,
    kind: i32,
    body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_loopback() {
        match SourceRconClient::connect("192.168.1.1", 25575, "pw", 1000) {
            Err(e) => assert!(e.message.contains("localhost")),
            Ok(_) => panic!("expected non-loopback rejection"),
        }
    }
}
