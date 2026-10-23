use sqlx::SqlitePool;

pub const PERMISSIONS: &[(&str, &str)] = &[
    ("device.read", "查看设备"),
    ("device.all", "查看全部设备"),
    ("device.write", "管理设备（改名、禁用、删除）"),
    ("device.connect", "发起远程控制"),
    ("user.read", "查看用户"),
    ("user.all", "查看全部用户"),
    ("user.write", "管理用户"),
    ("group.read", "查看分组"),
    ("group.write", "管理分组"),
    ("address_book.read", "查看地址簿"),
    ("address_book.write", "管理地址簿"),
    ("strategy.read", "查看策略"),
    ("strategy.write", "管理策略"),
    ("audit.read", "查看日志"),
    ("audit.manage", "管理日志（编辑备注、删除记录）"),
    ("deployment.manage", "管理客户端部署"),
    ("recording.read", "查看会话录像"),
    ("recording.manage", "管理会话录像"),
    ("relay.manage", "管理中继服务器"),
    ("settings.read", "查看系统设置"),
    ("settings.write", "修改系统设置"),
    ("role.manage", "管理管理员角色"),
];

const IMPLIED: &[(&str, &str)] = &[
    ("device.all", "device.read"),
    ("device.write", "device.read"),
    ("user.all", "user.read"),
    ("user.write", "user.read"),
    ("group.write", "group.read"),
    ("address_book.write", "address_book.read"),
    ("strategy.write", "strategy.read"),
    ("audit.manage", "audit.read"),
    ("recording.manage", "recording.read"),
    ("settings.write", "settings.read"),
];

pub fn has(permissions: &[String], permission: &str) -> bool {
    permissions
        .iter()
        .any(|item| item == "*" || item == permission)
}

fn expand_implied(permissions: &mut Vec<String>) {
    let granted: Vec<String> = permissions.clone();
    for (source, target) in IMPLIED {
        if granted.iter().any(|item| item == source) && !permissions.iter().any(|item| item == target) {
            permissions.push((*target).to_owned());
        }
    }
}

pub async fn load_permissions(
    pool: &SqlitePool,
    user_id: i64,
    is_admin: bool,
) -> Result<Vec<String>, sqlx::Error> {
    if is_admin {
        return Ok(vec!["*".to_owned()]);
    }
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT r.permissions FROM roles r
         JOIN user_roles ur ON ur.role_guid = r.guid
         WHERE ur.user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut merged: Vec<String> = Vec::new();
    for row in rows {
        if let Ok(items) = serde_json::from_str::<Vec<String>>(&row) {
            for item in items {
                if PERMISSIONS.iter().any(|(key, _)| *key == item) && !merged.contains(&item) {
                    merged.push(item);
                }
            }
        }
    }
    merged.sort();
    expand_implied(&mut merged);
    Ok(merged)
}

pub async fn load_roles(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT r.guid, r.name FROM roles r
         JOIN user_roles ur ON ur.role_guid = r.guid
         WHERE ur.user_id = ?
         ORDER BY r.name COLLATE NOCASE",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}
