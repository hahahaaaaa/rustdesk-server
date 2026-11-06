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

## OpenResty / Nginx 反向代理（HTTPS）

网页远控和文件传输要求 HTTPS，生产环境建议用 OpenResty（或 Nginx）终止 TLS，再转发到 `21114`。下面的配置可以直接放进 OpenResty 的 `http` 块，按注释替换域名、证书和上游地址即可。原生客户端走 WebSocket（`/ws/id`、`/ws/relay`）的完整示例见 [docs/nginx/rustdesk.conf](docs/nginx/rustdesk.conf)。

```nginx
# ---- 以下三项必须放在 http 块（不能放在 server 块里） ----
# 登录、单点登录、网页远控会话：每个 IP 每分钟 10 次
limit_req_zone $binary_remote_addr zone=rd_auth:10m rate=10r/m;
# 客户端上报接口（不需要登录）：每个 IP 每秒 20 次
limit_req_zone $binary_remote_addr zone=rd_client:10m rate=20r/s;
# 访问日志只记路径，不记查询参数，避免把 ticket=、token= 写进日志
log_format rd_safe '$remote_addr - $remote_user [$time_local] "$request_method $uri $server_protocol" '
                   '$status $body_bytes_sent "$http_user_agent"';

server {
    listen 80;
    server_name rd.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name rd.example.com;

    ssl_certificate     /etc/nginx/certs/fullchain.pem;
    ssl_certificate_key /etc/nginx/certs/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;

    access_log /var/log/nginx/rustdesk.access.log rd_safe;

    # 后端不发 HSTS，由这里统一加。CSP / X-Frame-Options 等由后端设置，这里不要重复配置。
    add_header Strict-Transport-Security "max-age=31536000" always;

    # 默认请求体上限，录像上传在下面单独放大
    client_max_body_size 2m;

    # 所有转发都带上这些头。X-Real-IP 必须用 $remote_addr 覆盖客户端传来的值：
    # 后端的登录限流和审计 IP 都依赖它，不覆盖的话可以伪造 IP 无限次猜密码。
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-Host $host;

    # 控制台和管理 API
    location / {
        proxy_pass http://127.0.0.1:21114;
    }

    # 网页远控、文件传输的 WebSocket。默认 60 秒超时会让画面静止时的连接断开。
    location ~ ^/api/web-control/(ws|relay)$ {
        proxy_pass http://127.0.0.1:21114;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
        proxy_buffering off;
    }

    # 登录、单点登录、创建网页远控会话：限流
    location = /api/login {
        limit_req zone=rd_auth burst=5 nodelay;
        proxy_pass http://127.0.0.1:21114;
    }
    location ^~ /api/oidc/ {
        limit_req zone=rd_auth burst=10 nodelay;
        proxy_pass http://127.0.0.1:21114;
    }
    location = /api/web-control/session {
        limit_req zone=rd_auth burst=10 nodelay;
        proxy_pass http://127.0.0.1:21114;
    }

    # 会话录像上传：后端单块上限 64MB
    location = /api/record {
        client_max_body_size 64m;
        limit_req zone=rd_client burst=40;
        proxy_pass http://127.0.0.1:21114;
        proxy_request_buffering off;
    }

    # 其他不需要登录的客户端接口：限流
    location ~ ^/api/(audit/|heartbeat$|sysinfo$) {
        limit_req zone=rd_client burst=40;
        proxy_pass http://127.0.0.1:21114;
    }
}
```

配合这份配置还需要做的事：

- **不要让 `21114` 直接暴露到公网**，只让 OpenResty 访问它。不然可以绕过代理，伪造 `X-Real-IP` 绕开登录限流。Docker 部署时把 `docker-compose.yml` 的端口映射改成 `"127.0.0.1:21114:21114"`；不用 Docker 时设置 `API_BIND=127.0.0.1`，或者用防火墙、安全组挡住这个端口。
- 改完端口映射后，客户端的 API 服务器要填 `https://rd.example.com`，不能再用 `http://IP:21114`。
- **单点登录的回调地址**：后端默认根据 `Host`、`X-Forwarded-Proto`、`X-Forwarded-Host` 推出回调地址，上面的配置已经带上这些头。更稳妥的做法是在控制台「集成 → OIDC」里直接填写回调地址。
- **只需改环境变量就能加强的两项**：`API_TOKEN_TTL_HOURS` 改为 8 到 24（默认 168 小时）；`API_CORS_ORIGINS` 改为 `https://rd.example.com`（默认 `*`）。
- 如果你在 OpenResty 里也配置了 `Content-Security-Policy`，浏览器会同时执行两份策略。这时 `script-src` 必须包含 `'unsafe-eval' 'wasm-unsafe-eval'`，否则网页远控无法运行。最简单的做法是不在代理上配置 CSP。
- `21115`～`21117`（ID 注册和中继）是 TCP/UDP 端口，不经过 HTTP 代理，要直接开放给客户端。

## 与上游的差异

- 上游只有 `hbbs` / `hbbr` / `rustdesk-utils`，本分支多了 `rustdesk-api` 和 Web 控制台；
- 上游镜像不支持 `RELAY` / `ENCRYPTED_ONLY` / `KEY_PUB` / `KEY_PRIV`，本分支的 s6 镜像支持；
- API 去掉了编译期 sqlx 查询，可离线交叉编译；`docker-classic/` 保留上游经典镜像的构建方式。

## 许可证

AGPL-3.0，见 [LICENSE](LICENSE)。
