#!/bin/bash

# 阿里云安全组配置脚本
# 使用方法: bash setup_security_group.sh <ACCESS_KEY_ID> <ACCESS_KEY_SECRET> <REGION_ID> <INSTANCE_ID>

set -e

ACCESS_KEY_ID="${1}"
ACCESS_KEY_SECRET="${2}"
REGION_ID="${3:-cn-beijing}"
INSTANCE_ID="${4:-i-2ze9kmxw4wuhffvt9b3t}"

echo "🔧 配置阿里云CLI..."

# 配置阿里云CLI（使用临时profile）
aliyun configure set \
  --profile tmp_security_group \
  --mode AK \
  --region "$REGION_ID" \
  --access-key-id "$ACCESS_KEY_ID" \
  --access-key-secret "$ACCESS_KEY_SECRET" \
  --language en

echo "📋 获取安全组ID..."
SECURITY_GROUP_ID=$(aliyun ecs DescribeInstances \
  --profile tmp_security_group \
  --RegionId "$REGION_ID" \
  --InstanceIds "[$INSTANCE_ID]" \
  --output json | grep -oP '"SecurityGroupId":\s*"\K[^"]+' | head -1)

if [ -z "$SECURITY_GROUP_ID" ]; then
    echo "❌ 无法获取安全组ID"
    exit 1
fi

echo "✅ 找到安全组: $SECURITY_GROUP_ID"

echo "🔓 添加 8081 端口规则..."
aliyun ecs AuthorizeSecurityGroup \
  --profile tmp_security_group \
  --RegionId "$REGION_ID" \
  --SecurityGroupId "$SECURITY_GROUP_ID" \
  --IpProtocol tcp \
  --PortRange 8081/8081 \
  --SourceCidrIp 0.0.0.0/0 \
  --Description "API Proxy Service"

echo "🔓 添加 8080 端口规则（可选）..."
aliyun ecs AuthorizeSecurityGroup \
  --profile tmp_security_group \
  --RegionId "$REGION_ID" \
  --SecurityGroupId "$SECURITY_GROUP_ID" \
  --IpProtocol tcp \
  --PortRange 8080/8080 \
  --SourceCidrIp 0.0.0.0/0 \
  --Description "API Proxy Direct" 2>/dev/null || echo "8080端口可能已存在"

echo "🧹 清理临时配置..."
rm -f ~/.aliyun/config.json

echo ""
echo "✅ 安全组配置完成！"
echo "📌 安全组ID: $SECURITY_GROUP_ID"
echo "🌐 已开放端口: 8081, 8080"
echo ""
echo "⏳ 请等待1-2分钟后，运行以下命令测试："
echo "   curl http://59.110.40.73:8081/health"
