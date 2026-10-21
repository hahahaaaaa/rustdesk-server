use std::collections::HashMap;
use std::io::{Error as IoError, ErrorKind};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::rejection::JsonRejection;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{now_secs, ONLINE_WINDOW_SECS};
use crate::AppState;

const TICKET_TTL: Duration = Duration::from_secs(120);
const MAX_FRAME_LENGTH: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TicketKind {
    Rendezvous,
    Relay,
}

#[derive(Debug, Clone)]
struct Ticket {
    kind: TicketKind,
    device_id: String,
    username: String,
    expires: Instant,
}

static TICKETS: OnceLock<Mutex<HashMap<String, Ticket>>> = OnceLock::new();

fn tickets() -> &'static Mutex<HashMap<String, Ticket>> {
    TICKETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn issue_ticket(kind: TicketKind, device_id: &str, username: &str) -> String {
    let now = Instant::now();
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let mut store = tickets().lock().unwrap_or_else(|err| err.into_inner());
    store.retain(|_, ticket| ticket.expires > now);
    store.insert(
        token.clone(),
        Ticket {
            kind,
            device_id: device_id.to_owned(),
            username: username.to_owned(),
            expires: now + TICKET_TTL,
        },
    );
    token
}

fn consume_ticket(token: &str, kind: TicketKind) -> Option<Ticket> {
    if token.is_empty() {
        return None;
    }
    let now = Instant::now();
    let mut store = tickets().lock().unwrap_or_else(|err| err.into_inner());
    let ticket = store.remove(token)?;
    (ticket.kind == kind && ticket.expires > now).then_some(ticket)
}

#[derive(Debug, Default, Deserialize)]
pub struct SessionRequest {
    #[serde(default)]
    pub guid: String,
}

pub async fn session(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<SessionRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let Json(request) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let guid = request.guid.trim();
    if guid.is_empty() {
        return Err(AppError::BadRequest("缺少设备标识。".to_owned()));
    }

    let row: Option<(String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT d.id, d.device_name, d.disabled, d.last_online
         FROM devices d
         WHERE d.guid = ?
           AND (? = 1 OR d.user = ?
                OR d.device_group_guid IN (
                    SELECT group_guid FROM user_device_groups WHERE user_id = ?
                ))",
    )
    .bind(guid)
    .bind(if auth.is_admin { 1_i64 } else { 0_i64 })
    .bind(&auth.username)
    .bind(auth.id)
    .fetch_optional(&state.db)
    .await?;

    let (device_id, device_name, disabled, last_online) =
        row.ok_or_else(|| AppError::NotFound("未找到设备或无权访问此设备。".to_owned()))?;
    if disabled != 0 {
        return Err(AppError::BadRequest(
            "设备已被禁用，无法发起远程控制。".to_owned(),
        ));
    }
    let public_key = state.cfg.server_public_key().ok_or_else(|| {
        tracing::error!(
            "web control requires the server public key; set API_SERVER_PUBLIC_KEY or \
             API_SERVER_PUBLIC_KEY_FILE (default /data/id_ed25519.pub)"
        );
        AppError::Internal("missing RustDesk server public key".to_owned())
    })?;

    let online = last_online.is_some_and(|ts| now_secs() - ts <= ONLINE_WINDOW_SECS);
    let ticket = issue_ticket(TicketKind::Rendezvous, &device_id, &auth.username);
    let relay_ticket = issue_ticket(TicketKind::Relay, &device_id, &auth.username);
    // hbbs/hbbr validate this against their configured key. With the default
    // key-pair mode that key is the server public key, so send it unless the
    // deployment uses a custom key.
    let license_key = state
        .cfg
        .license_key
        .clone()
        .unwrap_or_else(|| public_key.clone());

    super::audit(
        &state,
        &auth.username,
        "web_control.start",
        guid,
        &format!("网页远控 {device_id}"),
    )
    .await;

    Ok(Json(json!({
        "device": {
            "guid": guid,
            "id": device_id,
            "name": device_name,
            "online": online,
        },
        "public_key": public_key,
        "license_key": license_key,
        "ticket": ticket,
        "relay_ticket": relay_ticket,
        "ws_path": "/api/web-control/ws",
        "relay_path": "/api/web-control/relay",
    })))
}

#[derive(Debug, Default, Deserialize)]
pub struct TicketQuery {
    #[serde(default)]
    pub ticket: String,
}

pub async fn ws_proxy(
    State(state): State<AppState>,
    Query(query): Query<TicketQuery>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let ticket = consume_ticket(&query.ticket, TicketKind::Rendezvous)
        .ok_or_else(|| AppError::Unauthorized("网页远控会话已过期，请重新发起。".to_owned()))?;
    let target = state.cfg.rendezvous_addr.clone();
    tracing::info!(
        user = %ticket.username,
        device = %ticket.device_id,
        target = %target,
        "web control rendezvous proxy opened"
    );
    Ok(upgrade
        .max_message_size(MAX_FRAME_LENGTH)
        .max_frame_size(MAX_FRAME_LENGTH)
        .on_upgrade(move |socket| async move {
            if let Err(err) = proxy_socket(socket, target, Some(ticket.device_id)).await {
                tracing::warn!(error = %err, "web control rendezvous proxy closed");
            }
        }))
}

pub async fn relay_proxy(
    State(state): State<AppState>,
    Query(query): Query<TicketQuery>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let ticket = consume_ticket(&query.ticket, TicketKind::Relay)
        .ok_or_else(|| AppError::Unauthorized("网页远控中继会话已过期，请重新发起。".to_owned()))?;
    let target = state.cfg.relay_addr.clone();
    tracing::info!(
        user = %ticket.username,
        device = %ticket.device_id,
        target = %target,
        "web control relay proxy opened"
    );
    Ok(upgrade
        .max_message_size(MAX_FRAME_LENGTH)
        .max_frame_size(MAX_FRAME_LENGTH)
        .on_upgrade(move |socket| async move {
            if let Err(err) = proxy_socket(socket, target, None).await {
                tracing::warn!(error = %err, "web control relay proxy closed");
            }
        }))
}

async fn proxy_socket(
    mut socket: WebSocket,
    target: String,
    expected_device: Option<String>,
) -> Result<(), String> {
    let tcp = TcpStream::connect(&target)
        .await
        .map_err(|err| format!("connect {target}: {err}"))?;
    tcp.set_nodelay(true).ok();
    let (mut reader, mut writer) = tcp.into_split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    let read_task = tokio::spawn(async move {
        loop {
            match read_frame(&mut reader).await {
                Ok(frame) => {
                    if frame.is_empty() {
                        continue;
                    }
                    if tx.send(frame).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut first_message = true;
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(WsMessage::Binary(data))) => {
                        if data.len() > MAX_FRAME_LENGTH {
                            break;
                        }
                        if first_message {
                            first_message = false;
                            if let Some(expected) = expected_device.as_deref() {
                                if punch_hole_request_id(&data).as_deref() != Some(expected) {
                                    tracing::warn!("web control proxy rejected a mismatched device id");
                                    break;
                                }
                            }
                        }
                        let mut framed = Vec::with_capacity(data.len() + 4);
                        encode_frame(&data, &mut framed);
                        if writer.write_all(&framed).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Ok(WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Text(_))) => {}
                    Some(Err(_)) => break,
                }
            }
            frame = rx.recv() => {
                match frame {
                    Some(frame) => {
                        if socket.send(WsMessage::Binary(frame.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    read_task.abort();
    let _ = socket.send(WsMessage::Close(None)).await;
    Ok(())
}

async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let mut header = [0u8; 4];
    reader.read_exact(&mut header[..1]).await?;
    let head_len = ((header[0] & 0x3) + 1) as usize;
    if head_len > 1 {
        reader.read_exact(&mut header[1..head_len]).await?;
    }
    let mut length = 0usize;
    for (index, byte) in header.iter().take(head_len).enumerate() {
        length |= (*byte as usize) << (index * 8);
    }
    length >>= 2;
    if length > MAX_FRAME_LENGTH {
        return Err(IoError::new(ErrorKind::InvalidData, "frame too large"));
    }
    let mut data = vec![0u8; length];
    reader.read_exact(&mut data).await?;
    Ok(data)
}

fn encode_frame(data: &[u8], out: &mut Vec<u8>) {
    let length = data.len();
    if length <= 0x3f {
        out.push((length << 2) as u8);
    } else if length <= 0x3fff {
        out.extend_from_slice(&(((length << 2) as u16) | 1).to_le_bytes());
    } else if length <= 0x3f_ffff {
        let header = ((length << 2) as u32) | 2;
        out.extend_from_slice(&(header as u16).to_le_bytes());
        out.push((header >> 16) as u8);
    } else {
        out.extend_from_slice(&(((length as u32) << 2) | 3).to_le_bytes());
    }
    out.extend_from_slice(data);
}

fn punch_hole_request_id(data: &[u8]) -> Option<String> {
    let request = find_length_delimited_field(data, 8)?;
    let id = find_length_delimited_field(request, 1)?;
    String::from_utf8(id.to_vec()).ok()
}

fn find_length_delimited_field(data: &[u8], wanted: u64) -> Option<&[u8]> {
    let mut cursor = 0usize;
    while cursor < data.len() {
        let key = read_varint(data, &mut cursor)?;
        let field = key >> 3;
        let wire = (key & 7) as u8;
        match wire {
            0 => {
                read_varint(data, &mut cursor)?;
            }
            1 => {
                cursor = cursor.checked_add(8)?;
            }
            2 => {
                let length = read_varint(data, &mut cursor)? as usize;
                let end = cursor.checked_add(length)?;
                if end > data.len() {
                    return None;
                }
                if field == wanted {
                    return Some(&data[cursor..end]);
                }
                cursor = end;
            }
            5 => {
                cursor = cursor.checked_add(4)?;
            }
            _ => return None,
        }
        if cursor > data.len() {
            return None;
        }
    }
    None
}

fn read_varint(data: &[u8], cursor: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *data.get(*cursor)?;
        *cursor += 1;
        value |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                break;
            }
        }
        out
    }

    fn length_field(field: u64, payload: &[u8]) -> Vec<u8> {
        let mut out = varint((field << 3) | 2);
        out.extend(varint(payload.len() as u64));
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn extracts_punch_hole_request_id() {
        let mut request = length_field(1, b"123456789");
        request.extend(length_field(3, b"key"));
        let mut message = length_field(8, &request);
        message.extend(length_field(6, b"ignored"));
        assert_eq!(
            punch_hole_request_id(&message).as_deref(),
            Some("123456789")
        );
    }

    #[test]
    fn rejects_message_without_punch_hole_request() {
        let message = length_field(9, b"123456789");
        assert_eq!(punch_hole_request_id(&message), None);
    }

    #[test]
    fn frame_round_trip() {
        for length in [0usize, 1, 63, 64, 16_383, 16_384, 100_000] {
            let data = vec![0x5au8; length];
            let mut framed = Vec::new();
            encode_frame(&data, &mut framed);
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let decoded = runtime
                .block_on(read_frame(&mut framed.as_slice()))
                .unwrap();
            assert_eq!(decoded, data, "length {length}");
        }
    }
}
