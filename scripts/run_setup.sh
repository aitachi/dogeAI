#!/bin/bash

# 阿里云安全组快速配置脚本
# 请填入你的 AccessKey 信息

ACCESS_KEY_ID="LTAI5tFeEdevWoN95QVEX4mh"
ACCESS_KEY_SECRET="<请在这里填入你的AccessKey Secret>"
REGION_ID="cn-beijing"
INSTANCE_ID="i-2ze9kmxw4wuhffvt9b3t"

# ============= 执行配置 =============
echo "🔧 开始配置安全组..."

# 配置阿里云CLI
aliyun configure set \
  --profile tmp_api_proxy \
  --mode AK \
  --region "$REGION_ID" \
  --access-key-id "$ACCESS_KEY_ID" \
  --access-key-secret "$ACCESS_KEY_SECRET" \
  --language en

# 获取安全组ID
echo "📋 获取实例和安全组信息..."
SECURITY_GROUP_ID=$(aliyun ecs DescribeInstances \
  --profile tmp_api_proxy \
  --RegionId "$REGION_ID" \
  --InstanceIds "[$INSTANCE_ID]" \
  --output json 2>/dev/null | grep -oP '"SecurityGroupId":\s*"\K[^"]+' | head -1)

if [ -z "$SECURITY_GROUP_ID" ]; then
    echo "❌ 错误: 无法获取安全组ID，请检查AccessKey是否正确"
    aliyun configure delete --profile tmp_api_proxy 2>/dev/null
    exit 1
fi

echo "✅ 安全组ID: $SECURITY_GROUP_ID"

# 添加8081端口规则
echo "🔓 开放 8081 端口..."
aliyun ecs AuthorizeSecurityGroup \
  --profile tmp_api_proxy \
  --RegionId "$REGION_ID" \
  --SecurityGroupId "$SECURITY_GROUP_ID" \
  --IpProtocol tcp \
  --PortRange 8081/8081 \
  --SourceCidrIp 0.0.0.0/0 \
  --Description "Anthropic API Proxy" 2>/dev/null && echo "✅ 8081端口已开放" || echo "⚠️ 8081端口可能已存在"

# 添加8080端口规则
echo "🔓 开放 8080 端口..."
aliyun ecs AuthorizeSecurityGroup \
  --profile tmp_api_proxy \
  --RegionId "$REGION_ID" \
  --SecurityGroupId "$SECURITY_GROUP_ID" \
  --IpProtocol tcp \
  --PortRange 8080/8080 \
  --SourceCidrIp 0.0.0.0/0 \
  --Description "API Proxy Direct" 2>/dev/null && echo "✅ 8080端口已开放" || echo "⚠️ 8080端口可能已存在"

# 清理临时配置
echo "🧹 清理临时配置..."
aliyun configure delete --profile tmp_api_proxy 2>/dev/null
rm -f ~/.aliyun/config.json

echo ""
echo "✅✅✅ 配置完成！"
echo ""
echo "📋 配置摘要:"
echo "   实例ID: $INSTANCE_ID"
echo "   安全组ID: $SECURITY_GROUP_ID"
echo "   已开放端口: 8081, 8080"
echo ""
echo "⏳ 请等待1-2分钟生效后测试："
echo "   curl http://59.110.40.73:8081/health"
echo ""
