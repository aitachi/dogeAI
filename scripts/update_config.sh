#!/bin/bash
# 更新所有配置文件中的端口信息

# 1. 更新 README
sed -i 's|http://59.110.40.73:8081|http://59.110.40.73|g' /root/API-PROXY-README.md
sed -i 's|服务端口: `8081`|服务端口: `80` (标准HTTP)|g' /root/API-PROXY-README.md

# 2. 更新管理脚本
sed -i 's|http://59.110.40.73:8081|http://59.110.40.73|g' /root/api-proxy-admin.sh

# 3. 更新测试脚本
sed -i 's|http://59.110.40.73:8081|http://59.110.40.73|g' /root/test_api.sh

echo "✅ 配置文件已更新"
