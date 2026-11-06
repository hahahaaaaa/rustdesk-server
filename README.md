# RustDesk Server（含 Web 管理控制台）

基于 [rustdesk-server](https://github.com/rustdesk/rustdesk-server) 的下游分支：`hbbs` / `hbbr` 的协议与配置保持上游行为，在此基础上补上了 Web 管理控制台、浏览器网页远控和单镜像部署。

## 加了什么

- **Web 管理控制台**（`web-console/`）：Vue 3 + Vite，中文界面，和 API 同源跑在 `21114`。页面有设备、分组、地址簿、策略、日志、审计、用户、角色、部署、录制、中继、集成、设置、网页远控。
- **管理 API**（`api/`）：Rust + Axum + SQLite，JWT 登录、RBAC 权限、审计上报、录制分片、部署令牌。
- **浏览器网页远控**：设备菜单 → 「网页远控」（或 `/#/control/<guid>`），不装客户端直接操作。浏览器只连 API，由 API 代理到 hbbs/hbbr，视频用 WebCodecs 解码，支持多屏切换与鼠标坐标校正、卡顿自动恢复。要求 HTTPS + 新版 Chrome/Edge，且 hbbs 使用默认密钥对。
- **Pro 等效功能**：审计日志与告警邮件、客户端部署令牌、会话录制上传与回放、多中继管理、OIDC/LDAP/SMTP、邮件验证码登录、TOTP、自定义管理员角色。
- **单镜像多架构**：一个 s6 容器同时跑 hbbs + hbbr + api + console，数据都在 `/data`，镜像发布到 `ghcr.io/hahahaaaaa/rustdesk-server`（amd64/arm64/armv7）。
- **WebSocket 注册**：hbbs/hbbr 支持 `/ws/id`、`/ws/relay`，附 nginx 反代示例 `docs/nginx/rustdesk.conf`。

## 快速开始

```bash
# 先把 docker-compose.yml 里的 API_ADMIN_PASSWORD 改掉
docker compose up -d
```

打开 `http://<服务器>:21114/`，用 `admin` 登录。镜像公开，可直接拉取。

端口：`21114` 控制台/API（客户端心跳和配置也走这里）、`21115` NAT 类型测试、`21116` ID 注册（TCP+UDP）、`21117` 中继、`21118/21119` WebSocket（用反代或客户端开启 WebSocket 时才需要）。

首次启动若未设置 `API_ADMIN_PASSWORD`，随机密码只在日志里打印一次：

```bash
docker logs rustdesk-server 2>&1 | grep "created the initial administrator"
```

安全提示：`21114` 暴露到公网时，`/api/audit/*`、`/api/record`、`/api/devices/deploy` 这些面向客户端的接口用部署令牌或设备签名认证（沿用开源客户端协议，没法用会话），不需要匿名上报就把它限制在内网。

## 自己构建

```bash
cargo build --release                                  # hbbs / hbbr / rustdesk-utils
cargo build --release --manifest-path api/Cargo.toml   # rustdesk-api
cd web-console && npm ci && npm run build              # 控制台静态文件
```

`rustdesk-api` 默认监听 `21114`，把 `API_CONSOLE_DIR` 指向 `web-console/dist` 就能同端口提供控制台。

## 配置

`hbbs` / `hbbr` 支持命令行参数、环境变量和 `.env` 文件，`--help` 可看全部参数。常用的有 `-k` 密钥、`-p` 端口、`-r` 中继地址、`ALWAYS_USE_RELAY`、`RUST_LOG`。

完整变量（Docker 镜像变量、API 与控制台变量、网页远控参数）见 [docs/environment-variables.md](docs/environment-variables.md)。

## 反向代理（OpenResty / Nginx）

网页远控和文件传输要求 HTTPS。用 OpenResty 终止 TLS 后转发到 `21114`，完整配置见 [docs/nginx/rustdesk.conf](docs/nginx/rustdesk.conf)：已包含控制台与 API 转发、远控和文件传输所需的 WebSocket 升级与长超时、`X-Real-IP` 等请求头。

在此基础上还要注意：

- **`21114` 只对代理开放**，不要直接暴露公网。Docker 部署把端口映射改成 `"127.0.0.1:21114:21114"`，不用 Docker 时设 `API_BIND=127.0.0.1`。后端登录限流依赖 `X-Real-IP`，能直连就能伪造它绕过限流。
- 客户端 API 服务器填 `https://你的域名`；单点登录回调地址建议在控制台「集成 → OIDC」里直接填写。
- 建议再补上 HSTS（`add_header Strict-Transport-Security "max-age=31536000" always;`）和登录限流（`limit_req_zone`，用在 `/api/login`、`/api/oidc/`、`/api/web-control/session` 上）。
- 会话录像上传接口 `/api/record` 单块最大 64MB，OpenResty 默认 1MB 会返回 413，需要 `client_max_body_size 64m`。
- `API_TOKEN_TTL_HOURS` 建议 8～24（默认 168）；`API_CORS_ORIGINS` 改成自己的域名。
- `21115`～`21117` 是 TCP/UDP 端口，不经过 HTTP 代理，需直接开放给客户端。

## 与上游的差异

- 上游只有 `hbbs` / `hbbr` / `rustdesk-utils`，本分支多了 `rustdesk-api` 和 Web 控制台；
- 上游镜像不支持 `RELAY` / `ENCRYPTED_ONLY` / `KEY_PUB` / `KEY_PRIV`，本分支的 s6 镜像支持；
- API 去掉了编译期 sqlx 查询，可离线交叉编译；`docker-classic/` 保留上游经典镜像的构建方式。

## 许可证

AGPL-3.0，见 [LICENSE](LICENSE)。
