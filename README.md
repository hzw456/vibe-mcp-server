# Vibe MCP Server

AI Task Status Tracker - MCP Protocol Server

基于 Rust + Axum 框架实现的高性能 MCP 服务，支持 API Key 鉴权。

## ✨ 功能特性

- 🎯 **MCP 协议支持** - 兼容 Claude、Cursor 等 AI IDE 的 MCP 集成
- 🌐 **REST API** - 同时提供传统 HTTP API 接口
- 🔐 **API Key 鉴权** - 支持 X-API-Key 和 Bearer Token 认证
- 📊 **任务状态管理** - 实时跟踪 AI 任务进度
- 🔄 **心跳检测** - 自动清理超时任务
- 🚀 **高性能** - 基于 Tokio 异步运行时

## 🚀 快速开始

### Docker 部署（推荐）

```bash
# 1. 克隆并进入目录
git clone https://github.com/hzw456/vibe-mcp-server.git
cd vibe-mcp-server

# 2. 配置环境变量
cp .env.example .env
# 编辑 .env 设置 API_KEY

# 3. 构建并启动
docker-compose up -d

# 4. 查看日志
docker-compose logs -f vibe-mcp-server
```

### 本地运行

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 克隆并进入目录
git clone https://github.com/hzw456/vibe-mcp-server.git
cd vibe-mcp-server

# 3. 编译
cargo build --release

# 4. 运行
./target/release/vibe-mcp-server
```

### 使用 systemd

```bash
# 1. 编译并安装
cargo build --release
sudo mkdir -p /opt/vibe-mcp-server
sudo cp -r target/release/vibe-mcp-server /opt/vibe-mcp-server/bin/
sudo cp vibe-mcp.service /etc/systemd/system/
sudo cp .env.example /opt/vibe-mcp-server/.env

# 2. 配置并启动
sudo systemctl daemon-reload
sudo systemctl enable vibe-mcp
sudo systemctl start vibe-mcp

# 3. 查看状态
sudo systemctl status vibe-mcp
```

## 📡 API 文档

### REST API

| 端点 | 方法 | 说明 | 鉴权 |
|------|------|------|------|
| `/api/status` | GET | 获取所有任务状态 | 否 |
| `/api/task/report` | POST | 上报任务 | 是 |
| `/api/task/update_state` | POST | 更新任务状态 | 是 |
| `/api/task/update_progress` | POST | 更新任务进度 | 是 |
| `/api/task/update_state_by_path` | POST | 根据路径更新 | 是 |
| `/api/task/delete` | POST | 删除任务 | 是 |
| `/api/reset` | POST | 重置任务 | 是 |

### MCP 协议

MCP 端点：`POST /mcp`

#### tools/list

获取所有任务列表。

```json
{
  "method": "tools/call",
  "params": {
    "name": "list_tasks"
  }
}
```

#### update_task_status

更新任务状态。

```json
{
  "method": "tools/call",
  "params": {
    "name": "update_task_status",
    "arguments": {
      "task_id": "task-123",
      "status": "running"
    }
  }
}
```

#### update_task_progress

更新任务进度。

```json
{
  "method": "tools/call",
  "params": {
    "name": "update_task_progress",
    "arguments": {
      "task_id": "task-123",
      "estimated_duration_ms": 300000,
      "current_stage": "Analyzing code..."
    }
  }
}
```

## 🔐 鉴权

所有 API 端点（除 `/api/status` 和 `/mcp`）都需要鉴权。

### 使用 X-API-Key Header

```bash
curl -X POST http://localhost:3010/api/task/report \
  -H "Content-Type: application/json" \
  -H "X-API-Key: vibe-mcp-secret-key" \
  -d '{"task_id": "123", "name": "Test", "ide": "VSCode", "window_title": "test.py"}'
```

### 使用 Bearer Token

```bash
curl -X POST http://localhost:3010/api/task/report \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer vibe-mcp-secret-key" \
  -d '{"task_id": "123", "name": "Test", "ide": "VSCode", "window_title": "test.py"}'
```

## 🧪 测试

```bash
# 运行测试
cargo test

# 运行测试并查看覆盖率
cargo tarpaulin --out Xml
```

## 📦 项目结构

```
vibe-mcp-server/
├── src/
│   └── main.rs           # 主程序入口
├── Cargo.toml            # 依赖配置
├── Dockerfile           # Docker 构建配置
├── docker-compose.yml   # Docker Compose 配置
├── .env.example         # 环境变量示例
├── .gitignore          # Git 忽略配置
├── vibe-mcp.service    # systemd 服务配置
└── README.md          # 项目说明
```

## 🔧 配置项

| 环境变量 | 说明 | 默认值 |
|----------|------|--------|
| `API_KEY` | API 鉴权密钥 | `vibe-mcp-secret-key` |
| `HOST` | 绑定地址 | `0.0.0.0` |
| `PORT` | 端口号 | `3010` |
| `HEARTBEAT_TIMEOUT_MS` | 心跳超时时间（毫秒） | `5000` |

## 📄 License

MIT License
