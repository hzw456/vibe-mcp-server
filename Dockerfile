# 编译阶段
FROM rust:1.75-alpine AS builder

WORKDIR /app

# 安装编译依赖
RUN apk add --no-cache musl-dev openssl-dev

# 复制依赖文件
COPY Cargo.toml ./
COPY Cargo.lock ./

# 创建虚拟依赖目录（避免立即编译 src）
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# 预编译依赖（缓存）
RUN cargo build --release --locked

# 复制源代码并编译
COPY src ./src
RUN cargo build --release --locked

# ============ 运行阶段 ============
FROM alpine:3.19 AS runtime

# 安装运行时依赖
RUN apk add --no-cache openssl ca-certificates

# 创建非 root 用户
RUN addgroup -g 1000 app && adduser -u 1000 -G app -s /bin/sh -D app

# 复制编译产物
COPY --from=builder /app/target/release/vibe-mcp-server /usr/local/bin/

# 复制配置文件
COPY --from=builder /app/.env.example /app/.env.example

# 设置工作目录
WORKDIR /app

# 切换用户
USER app

# 暴露端口
EXPOSE 3010

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3010/health || exit 1

# 启动命令
CMD ["vibe-mcp-server", "--host", "0.0.0.0", "--port", "3010"]
