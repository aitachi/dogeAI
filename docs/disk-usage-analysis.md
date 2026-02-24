# 本机磁盘占用分析报告

> 生成时间: 2026-02-24
> 磁盘总容量: 40G
> 已使用: 17G (45%)
> 可用: 21G

---

## 一、磁盘占用总览

### 全局分布

| 目录 | 占用 | 占比 | 说明 |
|------|------|------|------|
| `/root` | 11G | 65% | 用户数据和项目代码 |
| `/usr` | 4.6G | 27% | 系统程序和库 |
| `/var` | 1.4G | 8% | 日志、缓存、运行数据 |
| **总计** | **17G** | **100%** | |

---

## 二、/root 目录占用详情 (11G)

### 2.1 Rust 项目编译产物 (7.8G) - **可清理**

| 项目 | target大小 | release | debug | 说明 |
|------|-----------|---------|-------|------|
| `api-gateway` | 2.8G | 837M | 2.0G | API网关主项目 |
| `aitachi/billing-system` | 2.4G | 833M | 1.5G | 计费网关 |
| `api-gateway-simple` | 1.7G | 972M | 750M | 简化网关 |
| `auth-system` | 711M | - | 711M | 认证系统 |
| `two-level-cache` | 294M | 294M | - | 两级缓存 |
| **总计** | **7.8G** | **2.9G** | **4.9G** | |

**说明**: `debug` 目录包含未优化的调试版本，生产环境可删除。

### 2.2 Rust 工具链缓存 (2.6G) - **可清理**

| 目录 | 大小 | 说明 |
|------|------|------|
| `.rustup/toolchains` | 1.3G | Rust 工具链(编译器、标准库) |
| `.npm/_cacache` | 992M | npm 包缓存 |
| `.cargo/registry` | 293M | Cargo crate 源码缓存 |
| **总计** | **2.6G** | |

**说明**:
- `.rustup`: 包含 Rust 编译器，删除后需重新下载
- `.npm`: npm 缓存，可安全删除
- `.cargo`: Cargo 包缓存，可安全删除

### 2.3 其他目录 (700M)

| 目录 | 大小 | 说明 |
|------|------|------|
| `frontend` | 68M | 前端项目 |
| `dogeai-venv` | 61M | Python 虚拟环境 |
| `.claude` | 155M | Claude Code 配置和插件 |
| `.openclaw` | 132M | OpenClaw 工具数据 |
| **总计** | **416M** | |

---

## 三、/usr 目录占用详情 (4.6G)

| 子目录 | 大小 | 说明 | 可清理 |
|--------|------|------|--------|
| `/usr/lib` | 2.9G | 系统库文件 | ❌ |
| `/usr/local` | 593M | 本地安装软件 | ⚠️ |
| `/usr/share` | 466M | 共享资源/文档 | ⚠️ |
| `/usr/bin` | 297M | 可执行程序 | ❌ |
| `/usr/src` | 157M | 源代码(可能是内核) | ⚠️ |

### 3.1 /usr/lib 细分 (2.9G)

| 目录 | 大小 | 说明 |
|------|------|------|
| `node_modules` | 1.3G | 全局 npm 包 |
| `firmware` | 488M | 硬件固件 |
| `x86_64-linux-gnu` | 451M | 系统库 |
| `llvm-18` | 179M | LLVM 编译器 |
| `modules` | 156M | 内核模块 |
| `python3/python3.12` | 247M | Python 运行时 |

---

## 四、/var 目录占用详情 (1.4G)

| 子目录 | 大小 | 说明 | 可清理 |
|--------|------|------|--------|
| `/var/log` | 643M | 系统日志 | ✅ |
| `/var/lib` | 469M | 运行时数据 | ❌ |
| `/var/cache` | 239M | 包管理器缓存 | ✅ |

### 4.1 日志文件详情 (643M)

| 日志类型 | 大小 | 说明 |
|----------|------|------|
| `journal` | 479M | systemd 日志 |
| `btmp` | 61M | 登录失败记录 |
| `iaas_monitor` | 44M | 云监控日志 |
| `auth.log` | 43M | 认证日志 |
| `sysstat` | 5.2M | 系统统计 |

---

## 五、磁盘占用汇总

### 按类别汇总

| 类别 | 大小 | 占比 | 状态 |
|------|------|------|------|
| **Rust debug 编译产物** | 4.9G | 29% | 🔴 可删除 |
| **Rust release 编译产物** | 2.9G | 17% | 🟡 保留生产用 |
| **Rust 工具链缓存** | 2.6G | 15% | 🟡 可重建 |
| **系统程序(/usr)** | 4.6G | 27% | 🟢 系统必需 |
| **系统日志(/var/log)** | 643M | 4% | 🟢 可定期清理 |
| **其他缓存** | 239M | 1% | 🟢 可清理 |
| **项目源码** | ~200M | 1% | 🟢 需保留 |
| **前端/虚拟环境** | ~130M | 1% | 🟡 按需保留 |
| **Claude/OpenClaw** | ~287M | 2% | 🟡 配置数据 |

---

## 六、清理建议

### 高优先级 - 立即可清理

| 项目 | 路径 | 大小 | 命令 |
|------|------|------|------|
| **debug 编译产物** | `*/target/debug` | 4.9G | `cargo clean --debug` |
| **npm 缓存** | `/root/.npm/_cacache` | 992M | `npm cache clean --force` |
| **Cargo 包缓存** | `/root/.cargo/registry` | 293M | `cargo cache --dir src` |
| **系统日志** | `/var/log/journal` | 479M | `journalctl --vacuum-size=100M` |
| **旧日志归档** | `/var/log/*.gz` | ~50M | `rm /var/log/*.gz` |

**可释放空间: ~7.7G**

### 中优先级 - 按需清理

| 项目 | 路径 | 大小 | 说明 |
|------|------|------|------|
| **未使用的 Rust 工具链** | `/root/.rustup/toolchains` | 1.3G | 保留当前版本即可 |
| **npm 全局包** | `/usr/lib/node_modules` | 1.3G | 清理未使用包 |
| **包管理器缓存** | `/var/cache` | 239M | apt/dnf 缓存 |

**额外可释放: ~2G**

### 低优先级 - 评估后清理

| 项目 | 大小 | 说明 |
|------|------|------|
| 停止服务的项目 target | 不定 | 清理已下线项目 |
| 旧的 Python 虚拟环境 | 61M | dogeai-venv 是否还用 |

---

## 七、清理命令参考

### 1. Rust 项目清理

```bash
# 清理所有 debug 版本 (推荐)
find /root -name "target" -type d -exec sh -c 'rm -rf "$1/debug"' _ {} \;

# 或使用 cargo clean (更彻底)
cd /root/api-gateway && cargo clean --debug
cd /root/api-gateway-simple && cargo clean --debug
cd /root/aitachi/billing-system && cargo clean --debug
cd /root/auth-system && cargo clean --debug
cd /root/two-level-cache && cargo clean --debug
```

### 2. 缓存清理

```bash
# npm 缓存
npm cache clean --force

# Cargo 缓存
cargo cache --dir src

# Rust toolchain 旧版本
rustup component list --installed
rustup target remove unused_targets
```

### 3. 系统日志清理

```bash
# systemd 日志保留 100M
journalctl --vacuum-size=100M

# 或保留最近 7 天
journalctl --vacuum-time=7d

# 清理旧日志
find /var/log -name "*.gz" -delete
find /var/log -name "*.1" -size +100M -delete
```

### 4. 包管理器缓存

```bash
# apt 缓存
apt-get clean
apt-get autoclean

# 或 (如果使用 dnf)
dnf clean all
```

---

## 八、清理后预估

### 当前状态
- 已使用: 17G (45%)
- 可用: 21G

### 执行高优先级清理后
- 可释放: ~7.7G
- 预计使用: 9.3G (23%)
- 预计可用: 30.7G

### 执行全部清理后
- 可释放: ~9.7G
- 预计使用: 7.3G (18%)
- 预计可用: 32.7G

---

## 九、监控建议

### 定期清理任务

```bash
# 添加到 crontab
# 每周清理日志
0 2 * * 0 journalctl --vacuum-time=7d

# 每月清理 npm 缓存
0 3 1 * * npm cache clean --force

# 每月清理 Cargo debug 构建产物
0 4 1 * * find /root -name "target" -type d -exec sh -c 'rm -rf "$1/debug"' _ {} \;
```

### 磁盘监控脚本

```bash
#!/bin/bash
# 磁盘使用超过 80% 时报警
USAGE=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')
if [ $USAGE -gt 80 ]; then
    echo "警告: 磁盘使用率 ${USAGE}%"
    # 发送通知...
fi
```

---

## 十、结论

本机 17G 磁盘占用主要由以下构成：

1. **Rust 编译产物 (7.8G)** - 占比最大，debug 版本可安全删除
2. **开发工具缓存 (2.6G)** - 可定期清理
3. **系统程序 (4.6G)** - 必需保留
4. **系统日志 (643M)** - 可定期清理

**建议**: 执行高优先级清理可释放 **7.7G** 空间，将磁盘使用率降至 23%。
