use lettre::transport::smtp::authentication::Credentials;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use rand::Rng;
use serde_json::json;

use crate::auth::create_email_challenge;
use crate::db;
use crate::error::AppError;
use crate::models::{now_secs, UserRow};
use crate::AppState;

const CHALLENGE_TTL_SECS: i64 = 600;
const MAX_ATTEMPTS: i64 = 5;

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub from_name: String,
    pub encryption: String,
}

impl SmtpConfig {
    pub async fn load(state: &AppState) -> Result<Option<Self>, AppError> {
        let enabled = db::get_setting(&state.db, "smtp_enabled").await?.as_deref() == Some("1");
        if !enabled {
            return Ok(None);
        }
        let host = setting(state, "smtp_host").await;
        let from = setting(state, "smtp_from").await;
        if host.is_empty() || from.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self {
            host,
            port: setting(state, "smtp_port").await.parse().unwrap_or(587),
            username: setting(state, "smtp_username").await,
            password: setting(state, "smtp_password").await,
            from,
            from_name: setting(state, "smtp_from_name").await,
            encryption: {
                let value = setting(state, "smtp_encryption").await;
                if value.is_empty() {
                    "starttls".to_owned()
                } else {
                    value
                }
            },
        }))
    }
}

async fn setting(state: &AppState, key: &str) -> String {
    db::get_setting(&state.db, key)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub async fn is_configured(state: &AppState) -> bool {
    matches!(SmtpConfig::load(state).await, Ok(Some(_)))
}

pub async fn send_mail(
    state: &AppState,
    to: &str,
    subject: &str,
    body: String,
) -> Result<(), String> {
    let config = SmtpConfig::load(state)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "SMTP 未配置或未启用。".to_owned())?;
    send_with_config(&config, to, subject, body).await
}

pub async fn send_with_config(
    config: &SmtpConfig,
    to: &str,
    subject: &str,
    body: String,
) -> Result<(), String> {
    let from = if config.from_name.is_empty() {
        config
            .from
            .parse()
            .map_err(|err| format!("发件人地址无效：{err}"))?
    } else {
        lettre::message::Mailbox::new(
            Some(config.from_name.clone()),
            config
                .from
                .parse()
                .map_err(|err| format!("发件人地址无效：{err}"))?,
        )
    };
    let message = Message::builder()
        .from(from)
        .to(to
            .parse()
            .map_err(|err| format!("收件人地址无效：{err}"))?)
        .subject(subject)
        .body(body)
        .map_err(|err| format!("构造邮件失败：{err}"))?;

    let builder = match config.encryption.as_str() {
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
            .port(config.port),
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .map_err(|err| format!("SMTP 配置错误：{err}"))?
            .port(config.port),
        _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
            .map_err(|err| format!("SMTP 配置错误：{err}"))?
            .port(config.port),
    };
    let builder = if config.username.is_empty() {
        builder
    } else {
        builder.credentials(Credentials::new(
            config.username.clone(),
            config.password.clone(),
        ))
    };
    builder
        .build()
        .send(message)
        .await
        .map_err(|err| format!("邮件发送失败：{err}"))?;
    Ok(())
}

pub async fn issue_email_challenge(
    state: &AppState,
    user: &UserRow,
) -> Result<String, AppError> {
    let code: String = {
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(0..1_000_000))
    };
    let (token, claims) = create_email_challenge(state, user, CHALLENGE_TTL_SECS)?;
    let now = now_secs();
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM email_challenges WHERE username = ? OR expires_at < ?")
        .bind(&user.username)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO email_challenges (jti, username, code, attempts, expires_at)
         VALUES (?, ?, ?, 0, ?)",
    )
    .bind(&claims.jti)
    .bind(&user.username)
    .bind(&code)
    .bind(claims.exp)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let body = format!(
        "您好 {name}：\n\n您正在登录 RustDesk 服务，验证码为 {code}。\n验证码 10 分钟内有效，请勿转发给他人。\n\n如果这不是您的操作，请忽略此邮件。\n",
        name = if user.display_name.is_empty() {
            &user.username
        } else {
            &user.display_name
        }
    );
    send_mail(state, &user.email, "RustDesk 登录验证码", body)
        .await
        .map_err(AppError::Internal)?;
    Ok(token)
}

pub async fn verify_email_challenge(
    state: &AppState,
    username: &str,
    token: &str,
    code: &str,
) -> Result<(), AppError> {
    let claims = crate::auth::decode_challenge(state, token, "email")?;
    if claims.sub != username {
        return Err(AppError::Unauthorized("邮箱验证会话与账号不匹配。".to_owned()));
    }
    let row: Option<(String, i64, i64)> = sqlx::query_as(
        "SELECT code, attempts, expires_at FROM email_challenges WHERE jti = ? AND username = ?",
    )
    .bind(&claims.jti)
    .bind(username)
    .fetch_optional(&state.db)
    .await?;
    let (expected, attempts, expires_at) = row.ok_or_else(|| {
        AppError::Unauthorized("邮箱验证码已失效，请重新登录。".to_owned())
    })?;
    if expires_at < now_secs() {
        sqlx::query("DELETE FROM email_challenges WHERE jti = ?")
            .bind(&claims.jti)
            .execute(&state.db)
            .await?;
        return Err(AppError::Unauthorized("邮箱验证码已过期，请重新登录。".to_owned()));
    }
    if attempts >= MAX_ATTEMPTS {
        sqlx::query("DELETE FROM email_challenges WHERE jti = ?")
            .bind(&claims.jti)
            .execute(&state.db)
            .await?;
        return Err(AppError::Unauthorized("验证码尝试次数过多，请重新登录。".to_owned()));
    }
    if code.trim() != expected {
        sqlx::query("UPDATE email_challenges SET attempts = attempts + 1 WHERE jti = ?")
            .bind(&claims.jti)
            .execute(&state.db)
            .await?;
        return Err(AppError::Unauthorized("验证码不正确。".to_owned()));
    }
    sqlx::query("DELETE FROM email_challenges WHERE jti = ?")
        .bind(&claims.jti)
        .execute(&state.db)
        .await?;
    Ok(())
}

pub async fn notify_alarm(state: &AppState, device_id: &str, alarm_type: i64, info: &str) {
    if !is_configured(state).await {
        return;
    }
    let recipients: Vec<String> = sqlx::query_scalar(
        "SELECT email FROM users
         WHERE is_admin = 1 AND status = 1 AND email <> '' AND email_alarm_notification = 1",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    if recipients.is_empty() {
        return;
    }
    let subject = format!("RustDesk 安全告警：{}", alarm_label(alarm_type));
    let body = format!(
        "RustDesk 服务产生了一条安全告警。\n\n设备 ID：{}\n告警类型：{}\n详情：{}\n时间：{}\n",
        if device_id.is_empty() { "未知" } else { device_id },
        alarm_label(alarm_type),
        info,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    for recipient in recipients {
        let body = body.clone();
        let subject = subject.clone();
        if let Err(err) = send_mail(state, &recipient, &subject, body).await {
            tracing::warn!(to = %recipient, error = %err, "failed to send alarm email");
        }
    }
}

pub fn alarm_label(alarm_type: i64) -> &'static str {
    match alarm_type {
        0 => "IP 白名单拦截",
        1 => "登录尝试超过 30 次",
        2 => "一分钟内尝试 6 次登录",
        6 => "IPv6 前缀登录尝试异常",
        7 => "终端系统登录退避",
        8 => "终端系统登录并发超限",
        9 => "会话范围越权",
        10 => "设备 ID 白名单拦截",
        _ => "未知告警",
    }
}

pub fn alarm_email_json(alarm_type: i64) -> serde_json::Value {
    json!({ "type": alarm_type, "label": alarm_label(alarm_type) })
}
