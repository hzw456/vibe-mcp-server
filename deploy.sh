#!/bin/bash

# Vibe MCP Server 部署脚本
# 用法: ./deploy.sh

set -e

# 配置
SERVER_IP="43.143.135.49"
SERVER_USER="root"
SERVER_DIR="/root"
SERVER_PORT="3011"
LOCAL_BINARY="/Users/zwhao/clawd/vibe-mcp-server/target/release/vibe-mcp-server"
SERVER_BINARY="$SERVER_DIR/vibe-mcp-server"

echo "===================================="
echo "   Vibe MCP Server 部署脚本"
echo "===================================="
echo ""

# 检查本地二进制
if [ ! -f "$LOCAL_BINARY" ]; then
    echo "❌ 错误: 本地二进制不存在: $LOCAL_BINARY"
    echo "请先编译: cd /Users/zwhao/clawd/vibe-mcp-server && cargo build --release"
    exit 1
fi

echo "✅ 本地二进制存在: $LOCAL_BINARY"
echo ""

# 上传二进制
echo "📤 上传二进制到服务器..."
scp "$LOCAL_BINARY" "$SERVER_USER@$SERVER_IP:$SERVER_BINARY"
echo "✅ 上传完成"
echo ""

# 部署
echo "🚀 部署到服务器..."
ssh "$SERVER_USER@$SERVER_IP" << EOF
    cd $SERVER_DIR
    
    # 停止旧进程
    if pgrep -f vibe-mcp-server > /dev/null; then
        echo "🛑 停止旧进程..."
        pkill vibe-mcp-server || true
        sleep 2
    fi
    
    # 设置权限
    chmod +x vibe-mcp-server
    
    # 停止旧进程(确保)
    pkill vibe-mcp-server || true
    
    # 启动新进程
    echo "🚀 启动服务..."
    nohup ./vibe-mcp-server --host 0.0.0.0 --port $SERVER_PORT > /tmp/vibe.log 2>&1 &
    
    # 等待启动
    sleep 3
    
    # 验证
    if curl -s http://localhost:$SERVER_PORT/health > /dev/null; then
        echo "✅ 服务启动成功!"
        curl -s http://localhost:$SERVER_PORT/health
        echo ""
    else
        echo "❌ 服务启动失败!"
        echo "日志:"
        cat /tmp/vibe.log
    fi
EOF

echo ""
echo "===================================="
echo "   部署完成!"
echo "===================================="
echo ""
echo "验证:"
echo "  curl http://$SERVER_IP:$SERVER_PORT/health"
echo ""
echo "API 端点:"
echo "  POST http://$SERVER_IP:$SERVER_PORT/api/auth/register"
echo "  POST http://$SERVER_IP:$SERVER_PORT/api/auth/login"
echo "  GET  http://$SERVER_IP:$SERVER_PORT/api/status"
