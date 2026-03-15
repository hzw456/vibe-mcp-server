# Vibe MCP Server 手动部署指南

## 方法 1: 使用 FinalShell 或其他 SSH 客户端

### 步骤 1: 下载二进制文件
从 Mac 下载文件:
```bash
# 使用 FinalShell、Cyberduck 或其他 SFTP 工具
# 服务器: 43.143.135.49
# 用户名: root
# 密码: (联系管理员获取)

# 或者使用 scp (如果已配置密钥):
scp /Users/zwhao/clawd/vibe-mcp-server/target/release/vibe-mcp-server root@43.143.135.49:/root/
```

### 步骤 2: SSH 连接并部署
```bash
# 1. SSH 连接
ssh root@43.143.135.49

# 2. 停止旧进程
pkill vibe-mcp-server || true
sleep 2

# 3. 上传文件 (如果还没上传)
# 从你的 Mac:
scp /Users/zwhao/clawd/vibe-mcp-server/target/release/vibe-mcp-server root@43.143.135.49:/root/

# 4. 在服务器上:
chmod +x vibe-mcp-server

# 5. 启动服务
nohup ./vibe-mcp-server > server.log 2>&1 &

# 6. 验证
curl http://localhost:3010/health
# 应该返回: OK
```

### 步骤 3: 测试 API
```bash
# 测试健康检查
curl http://43.143.135.49:3010/health

# 测试注册
curl -X POST http://43.143.135.49:3010/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test@vibe.app","password":"test123"}'

# 测试登录
curl -X POST http://43.143.135.49:3010/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@vibe.app","password":"test123"}'
```

## 方法 2: 使用 FTP/SFTP

1. 使用 FileZilla、Cyberduck 或 FinalShell
2. 连接信息:
   - 协议: SFTP
   - 服务器: 43.143.135.49
   - 端口: 22
   - 用户名: root
   - 密码: (联系管理员)

3. 上传文件:
   - 源: /Users/zwhao/clawd/vibe-mcp-server/target/release/vibe-mcp-server
   - 目标: /root/vibe-mcp-server

4. 然后 SSH 连接并启动服务

## 方法 3: 使用 GitHub Actions (推荐)

创建 `.github/workflows/deploy.yml`:

```yaml
name: Deploy to Server

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup SSH
        uses: appleboy/ssh-action@v0.1.3
        with:
          host: ${{ secrets.SERVER_HOST }}
          username: root
          key: ${{ secrets.SERVER_SSH_KEY }}
          script: |
            cd /root
            pkill vibe-mcp-server || true
            # 上传并启动新版本
```

## 当前状态

- ✅ 二进制已编译: `/Users/zwhao/clawd/vibe-mcp-server/target/release/vibe-mcp-server`
- ✅ 部署脚本已创建: `/Users/zwhao/clawd/vibe-mcp-server/deploy.sh`
- ❌ SSH 访问被拒绝 - 需要手动部署

## 快速测试 (本地服务器)

如果暂时无法部署到云端，可以在本地测试:

```bash
# 启动本地服务器
cd /Users/zwhao/clawd/vibe-mcp-server
./target/release/vibe-mcp-server &

# 测试
curl http://localhost:3010/health
```

## 联系

如有问题，请联系管理员获取 SSH 访问权限。
