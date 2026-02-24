# 编译错误修复指南

## 当前状态
- 总错误数: 22个
- 已修复: 7个
- 待修复: 22个

## 待修复错误列表

### 1. moka 缓存异步调用 (约10个)
**位置**: `src/cache.rs`
**问题**: moka v0.12 的 `get()` 和 `insert()` 是异步方法, 需要 `.await`

**修复**:
```rust
// ❌ 错误
if let Some(cached) = self.local_cache.get(token) {

// ✅ 正确
if let Some(cached) = self.local_cache.get().await {

// ❌ 错误
self.local_cache.insert(token.to_string(), cached.clone());

// ✅ 正确
self.local_cache.insert(token.to_string(), cached.clone()).await;
```

**需要修复的位置**:
- 第90行: `get(token)` → `get().await`
- 第120行: `insert(...)` → `insert(...).await`
- 第218行: `invalidate(token)` → `invalidate().await`
- 第277行: `invalidate_all()` → `invalidate_all().await`

### 2. 中间件工厂返回类型 (约4个)
**位置**: `src/middleware.rs`
**问题**: 异步闭包返回类型不匹配

**修复**:
```rust
// ❌ 错误
pub fn require_model_permission(model: &'static str)
    -> impl Fn(Request, Next) -> Result<Response, StatusCode> + Clone
{
    move |req: Request, next: Next| {
        async move {
            // ...
        }
    }
}

// ✅ 正确
pub fn require_model_permission(model: &'static str)
    -> impl Fn(Request, Next) -> impl Future<Output = Result<Response, StatusCode>> + Clone
{
    move |req: Request, next: Next| {
        let model = model.clone();
        async move {
            // ...
        }
    }
}
```

### 3. sqlx 类型推断 (约5个)
**位置**: `src/service.rs`
**问题**: sqlx 宏需要显式类型或使用 `?` 操作符

**修复**:
```rust
// ❌ 错误
let row = sqlx::query_as!(
    UserInfo,
    "SELECT ...",
    user_id
)
.fetch_optional(&self.db)
.await?;

// ✅ 正确 (添加类型标注)
let row: Option<UserInfo> = sqlx::query_as!(
    UserInfo,
    "SELECT ...",
    user_id
)
.fetch_optional(self.db.as_ref())
.await?;
```

### 4. axum FromRequestParts 生命周期 (1个)
**位置**: `src/middleware.rs:273`
**问题**: trait 定义生命周期不匹配

**修复**:
```rust
// 更新 axum 到 0.8 或使用正确的生命周期参数
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts<'life0, 'life1, 'life2, 'life3>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> Result<Self, Self::Rejection> {
        // ...
    }
}
```

### 5. 其他类型错误 (约2个)

## 快速修复命令

### 方案A: 更新依赖版本 (推荐)
```bash
# 更新到最新版本
cargo update

# 或手动指定版本
# Cargo.toml
jsonwebtoken = "10"
axum = "0.8"
redis = "1.0"
deadpool-redis = "0.22"
```

### 方案B: 锁定当前版本
```bash
# 生成 Cargo.lock
cargo update

# 锁定特定版本
# Cargo.toml
[dependencies]
axum = "=0.7.9"
sqlx = "=0.7.4"
```

## 推荐修复顺序

1. **立即执行**:
   ```bash
   # 更新 Cargo.toml 依赖版本
   sed -i 's/axum = "0.7"/axum = "0.8"/' Cargo.toml
   sed -i 's/jsonwebtoken = "9.2"/jsonwebtoken = "10"/' Cargo.toml
   sed -i 's/redis = "0.24"/redis = "1.0"/' Cargo.toml
   sed -i 's/deadpool-redis = "0.14"/deadpool-redis = "0.22"/' Cargo.toml

   # 重新编译
   cargo clean && cargo build
   ```

2. **手动修复 moka 调用**:
   - 在 `src/cache.rs` 中添加 `.await` 到所有 `get()` 调用
   - 在 `src/cache.rs` 中添加 `.await` 到所有 `insert()` 调用

3. **修复中间件**:
   - 使用 `async move` 闭包的正确返回类型

## 预计修复时间
- 更新依赖: 5分钟
- 修复 moka 调用: 10分钟
- 修复中间件: 15分钟
- 修复 sqlx: 10分钟
- **总计**: ~40分钟

## 自动化修复脚本

创建 `fix_errors.sh`:
```bash
#!/bin/bash
set -e

echo "修复 moka 缓存调用..."
sed -i 's/self\.local_cache\.get(token)/self.local_cache.get().await/' src/cache.rs
sed -i 's/self\.local_cache\.insert(\(.*\));/self.local_cache.insert(\1).await;/' src/cache.rs
sed -i 's/self\.local_cache\.invalidate(token)/self.local_cache.invalidate().await/' src/cache.rs
sed -i 's/self\.local_cache\.invalidate_all()/self.local_cache.invalidate_all().await/' src/cache.rs

echo "更新依赖版本..."
sed -i 's/axum = "0.7"/axum = "0.8"/' Cargo.toml
sed -i 's/jsonwebtoken = "9.2"/jsonwebtoken = "10"/' Cargo.toml

echo "重新编译..."
cargo clean && cargo build

echo "✅ 修复完成!"
```

运行:
```bash
chmod +x fix_errors.sh
./fix_errors.sh
```

## 验证修复

```bash
# 检查编译
cargo check

# 运行测试
cargo test

# 构建发布版本
cargo build --release
```

## 状态

- [ ] 更新依赖版本
- [ ] 修复 moka 异步调用
- [ ] 修复中间件工厂
- [ ] 修复 sqlx 查询
- [ ] 验证编译通过
- [ ] 运行测试

## 参考资料

- [moka v0.12 文档](https://docs.rs/moka/0.12.0/moka/)
- [axum v0.8 升级指南](https://docs.rs/axum/0.8.0/axum/index.html#upgrading)
- [sqlx v0.8 发布说明](https://github.com/launchbadge/sqlx/releases/tag/v0.8.0)
