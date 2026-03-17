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
- 💾 **持久化存储** - 支持 MySQL 数据库持久化存储

## 🚀 快速开始

### Docker 部署（推荐）

```bash
# 1. 克隆并进入目录
git clone https://github.com/hzw456/vibe-mcp-server.git
cd vibe-mcp-server

# 2. 配置环境变量
cp .env.example .env
# 编辑 .env 设置 API_KEY 和数据库配置

# 3. 构建并启动
docker-compose up -d

# 4. 查看日志
docker-compose logs -f vibe-mcp-server
```

### 本地运行（无数据库）

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

### 本地运行（使用 MySQL）

```bash
# 1. 安装并启动 MySQL
# macOS:
brew install mysql
brew services start mysql

# 2. 创建数据库
mysql -u root -p
CREATE DATABASE vibe_db;
EXIT;

# 3. 运行迁移脚本
mysql -u root -p vibe_db < migrations/001_initial_schema.sql

# 4. 配置环境变量
export USE_DATABASE=true
export DATABASE_URL="mysql://root:password@localhost:3306/vibe_db"

# 5. 编译并运行
cargo build --release
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

## 📚 数据库配置

### 环境变量

| 环境变量 | 说明 | 默认值 | 必填 |
|----------|------|--------|------|
| `USE_DATABASE` | 是否启用 MySQL | `false` | 否 |
| `DATABASE_URL` | MySQL 连接字符串 | `mysql://vibe:vibe@localhost:3306/vibe_db` | 当 USE_DATABASE=true 时 |
| `API_KEY` | API 鉴权密钥 | `vibe-mcp-secret-key` | 否 |
| `HOST` | 绑定地址 | `0.0.0.0` | 否 |
| `PORT` | 端口号 | `3010` | 否 |
| `JWT_SECRET` | JWT 密钥 | 自动生成 | 否 |
| `JWT_EXPIRY_HOURS` | JWT 过期时间（小时） | `24` | 否 |

### 数据库连接字符串格式

```
mysql://username:password@host:port/database_name
```

示例：
- 本地开发: `mysql://root:password@localhost:3306/vibe_db`
- Docker: `mysql://vibe:vibe@mysql:3306/vibe_db`

### 数据库表结构

#### vibe_users
用户表，存储用户账户信息。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | VARCHAR(36) | 用户唯一 ID (UUID) |
| email | VARCHAR(255) | 邮箱地址（唯一） |
| password_hash | VARCHAR(255) | bcrypt 加密的密码 |
| is_verified | BOOLEAN | 邮箱是否已验证 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

#### vibe_tasks
任务表，存储 AI 任务状态信息。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | VARCHAR(36) | 任务唯一 ID (UUID) |
| user_id | VARCHAR(36) | 所属用户 ID |
| parent_task_id | VARCHAR(36) | 父任务 ID（支持子任务） |
| name | VARCHAR(255) | 任务名称 |
| status | ENUM | 任务状态 |
| current_stage | TEXT | 当前阶段描述 |
| source | VARCHAR(50) | 任务来源 |
| ide | VARCHAR(255) | 使用的 IDE |
| window_title | VARCHAR(500) | 窗口标题 |
| project_path | VARCHAR(1000) | 项目路径 |
| active_file | VARCHAR(1000) | 当前活动文件 |
| is_focused | BOOLEAN | 是否聚焦 |
| start_time | BIGINT | 开始时间（毫秒） |
| end_time | BIGINT | 结束时间（毫秒） |
| last_heartbeat | BIGINT | 最后心跳时间 |
| estimated_duration_ms | BIGINT | 预估持续时间 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

#### vibe_task_stages
任务阶段历史表，记录每次阶段切换的起止时间与耗时。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGINT | 自增主键 |
| task_id | VARCHAR(36) | 关联任务 ID |
| stage | TEXT | 阶段名称 |
| started_at | BIGINT | 阶段开始时间（毫秒） |
| ended_at | BIGINT | 阶段结束时间（毫秒） |
| duration | BIGINT | 阶段耗时（毫秒） |
| created_at | BIGINT | 创建时间 |
| updated_at | BIGINT | 更新时间 |

#### vibe_verification_codes
验证码表，存储邮箱验证码。

| 字段 | 类型 | 说明 |
|------|------|------|
| email | VARCHAR(255) | 邮箱地址（主键） |
| code | VARCHAR(10) | 验证码 |
| expires_at | TIMESTAMP | 过期时间 |
| created_at | TIMESTAMP | 创建时间 |

### 运行迁移

#### 方式一：使用 SQL 文件

```bash
mysql -u root -p vibe_db < migrations/001_initial_schema.sql
```

#### 方式二：自动初始化

当 `USE_DATABASE=true` 时，服务器启动时会自动创建表结构。

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

本项目包含全面的单元测试，覆盖模型、服务和 API 集成。

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test models::         # 模型测试
cargo test services::       # 服务测试
cargo test api::            # API 测试
cargo test config::         # 配置测试

# 运行详细输出
cargo test -- --nocapture

# 运行单个测试
cargo test test_user_serialization
```

### 测试覆盖范围

#### 模型测试 (`tests/models.rs`)
- User 模型序列化/反序列化
- Task 模型创建和状态转换
- Claims (JWT) 结构验证
- VerificationCode 序列化

#### 配置测试 (`tests/config.rs`)
- 从环境变量加载配置
- 默认值验证
- JWT 过期时间验证
- 数据库配置验证

#### 服务测试 (`tests/services.rs`)
- JWT Token 生成和验证
- 密码哈希 (bcrypt)
- 验证码生成
- Task 创建逻辑

#### API 集成测试 (`tests/api.rs`)
- 完整认证流程：注册 → 验证 → 登录
- Task CRUD 操作（含 JWT 认证）
- 错误处理（无效 Token、缺失字段）

### 测试数据

测试使用的默认数据：
- **测试用户**: `testuser@vibe.app` / `testpass123`
- **测试任务**: `test-task-001`, `test-task-002`, `test-task-003`
- **JWT Secret**: `test-secret` (仅用于测试)

### 测试覆盖率

```bash
# 查看测试覆盖率
cargo tarpaulin --out Xml

# 或使用 HTML 输出
cargo tarpaulin --out Html
```

目标：核心业务逻辑覆盖率 > 80%

## 📦 项目结构

```
vibe-mcp-server/
├── src/
│   ├── main.rs           # 主程序入口
│   ├── lib.rs           # 库入口
│   ├── models/          # 数据模型
│   │   ├── user.rs      # User 模型
│   │   ├── task.rs      # Task 模型（含 parent_task_id）
│   │   └── auth.rs      # Claims (JWT)
│   ├── services/        # 业务逻辑
│   │   ├── auth_service.rs
│   │   ├── user_service.rs
│   │   └── task_service.rs
│   ├── api/             # API 处理器
│   │   ├── auth.rs
│   │   ├── tasks.rs
│   │   └── mcp.rs
│   ├── config/          # 配置
│   │   └── app.rs
│   ├── utils/           # 工具函数
│   │   └── helpers.rs
│   └── db/              # 数据库模块
│       ├── mod.rs       # 数据库连接和初始化
│       ├── user_repository.rs
│       ├── task_repository.rs
│       └── verification_repository.rs
├── migrations/          # 数据库迁移
│   └── 001_initial_schema.sql
├── tests/               # 单元测试
│   ├── models.rs        # 模型测试
│   ├── config.rs        # 配置测试
│   ├── services.rs      # 服务测试
│   ├── api.rs           # API 测试
│   └── test_utils.rs    # 测试工具
├── Cargo.toml           # 依赖配置
└── README.md            # 项目说明
```

## 🔧 配置项

| 环境变量 | 说明 | 默认值 |
|----------|------|--------|
| `API_KEY` | API 鉴权密钥 | `vibe-mcp-secret-key` |
| `HOST` | 绑定地址 | `0.0.0.0` |
| `PORT` | 端口号 | `3010` |
| `USE_DATABASE` | 启用 MySQL | `false` |
| `DATABASE_URL` | MySQL 连接字符串 | `mysql://vibe:vibe@localhost:3306/vibe_db` |
| `HEARTBEAT_TIMEOUT_MS` | 心跳超时时间（毫秒） | `5000` |

## 📄 License

MIT License
