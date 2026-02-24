# API网关 Dockerfile

# 构建阶段
FROM rust:1.75-slim as builder

WORKDIR /app

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制Cargo文件
COPY Cargo.toml Cargo.lock ./

# 创建虚拟src目录以利用Docker缓存
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs

# 构建依赖（利用Docker层缓存）
RUN cargo build --release && \
    rm -rf src

# 复制实际源代码
COPY src ./src
COPY migrations ./migrations

# 构建应用
RUN touch src/main.rs && \
    cargo build --release

# 运行阶段
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/api-gateway /usr/local/bin/api-gateway

# 复制配置文件
COPY config.toml /etc/api-gateway/config.toml

# 创建非root用户
RUN useradd -m -u 1000 gateway && \
    chown -R gateway:gateway /home/gateway

USER gateway

WORKDIR /home/gateway

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 启动应用
CMD ["api-gateway"]
