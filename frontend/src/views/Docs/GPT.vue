<template>
  <div class="docs-page">
    <div class="docs-container">
      <!-- Sidebar -->
      <aside class="docs-sidebar">
        <h3>GPT API 文档</h3>
        <nav class="docs-nav">
          <a v-for="section in sections" :key="section.id"
             :class="['nav-item', { active: activeSection === section.id }]"
             @click="activeSection = section.id">
            {{ section.title }}
          </a>
        </nav>
      </aside>

      <!-- Content -->
      <main class="docs-content">
        <!-- Quick Start -->
        <section v-if="activeSection === 'quickstart'" class="docs-section">
          <h1>快速开始</h1>
          <p class="lead">使用您的 API Key 快速接入 GPT 模型</p>

          <div class="code-block">
            <div class="code-header">环境变量配置</div>
            <pre><code>export OPENAI_API_KEY="sk-xxxxxx"
export OPENAI_BASE_URL="https://115.190.62.87/v1"</code></pre>
          </div>

          <h3>Python 示例</h3>
          <div class="code-block">
            <pre><code>from openai import OpenAI

client = OpenAI(
    api_key="sk-xxxxxx",
    base_url="https://115.190.62.87/v1"
)

response = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello!"}
    ]
)

print(response.choices[0].message.content)</code></pre>
          </div>

          <h3>Node.js 示例</h3>
          <div class="code-block">
            <pre><code>import OpenAI from 'openai'

const client = new OpenAI({
  apiKey: 'sk-xxxxxx',
  baseURL: 'https://115.190.62.87/v1'
})

const response = await client.chat.completions.create({
  model: 'gpt-4o',
  messages: [
    { role: 'system', content: 'You are a helpful assistant.' },
    { role: 'user', content: 'Hello!' }
  ]
})

console.log(response.choices[0].message.content)</code></pre>
          </div>

          <h3>curl 示例</h3>
          <div class="code-block">
            <pre><code>curl https://115.190.62.87/v1/chat/completions \
  -H "x-api-key: sk-xxxxxx" \
  -H "content-type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "Hello!"}
    ]
  }'</code></pre>
          </div>
        </section>

        <!-- Models -->
        <section v-if="activeSection === 'models'" class="docs-section">
          <h1>支持的模型</h1>
          <p class="lead">我们提供 GPT 系列所有主流模型的中转服务</p>

          <div class="model-list">
            <div class="model-card" v-for="model in models" :key="model.id">
              <div class="model-header">
                <h3>{{ model.name }}</h3>
                <span :class="['tier-badge', model.tier]">{{ model.tier }}</span>
              </div>
              <p class="model-desc">{{ model.description }}</p>
              <div class="model-specs">
                <span class="spec">上下文: {{ model.context }}</span>
                <span class="spec">输入: ¥{{ model.inputPrice }}/M tokens</span>
                <span class="spec">输出: ¥{{ model.outputPrice }}/M tokens</span>
              </div>
            </div>
          </div>
        </section>

        <!-- Compatibility -->
        <section v-if="activeSection === 'compat'" class="docs-section">
          <h1>兼容性说明</h1>
          <p class="lead">我们的 API 完全兼容 OpenAI API 格式</p>

          <div class="alert alert-info">
            所有支持 OpenAI API 的工具和库都可以直接使用，只需修改 base_url 即可。
          </div>

          <h3>支持的客户端</h3>
          <div class="client-list">
            <div class="client-item" v-for="client in compatibleClients" :key="client.name">
              <h4>{{ client.name }}</h4>
              <p>{{ client.description }}</p>
            </div>
          </div>
        </section>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const activeSection = ref('quickstart')

const sections = [
  { id: 'quickstart', title: '快速开始' },
  { id: 'models', title: '模型列表' },
  { id: 'compat', title: '兼容性' }
]

const models = [
  {
    id: 'gpt-4o',
    name: 'GPT-4o',
    tier: 'pro',
    description: '最新旗舰模型，多模态能力强',
    context: '128K',
    inputPrice: '3.5',
    outputPrice: '14.0'
  },
  {
    id: 'gpt-4o-mini',
    name: 'GPT-4o Mini',
    tier: 'base',
    description: '轻量级模型，性价比高',
    context: '128K',
    inputPrice: '0.15',
    outputPrice: '0.6'
  },
  {
    id: 'gpt-4-turbo',
    name: 'GPT-4 Turbo',
    tier: 'free',
    description: '高性能模型，适合复杂任务',
    context: '128K',
    inputPrice: '10.0',
    outputPrice: '30.0'
  }
]

const compatibleClients = [
  {
    name: 'OpenAI Python SDK',
    description: '官方 Python 库，设置 base_url 参数即可'
  },
  {
    name: 'OpenAI Node.js SDK',
    description: '官方 JavaScript/TypeScript 库'
  },
  {
    name: 'LangChain',
    description: '流行的 AI 应用开发框架'
  },
  {
    name: 'LlamaIndex',
    description: '数据框架，用于构建 LLM 应用'
  },
  {
    name: 'Cursor / Windsurf',
    description: 'AI 代码编辑器，直接配置自定义端点'
  }
]
</script>

<style scoped>
.docs-page {
  min-height: 100vh;
  background: #f9fafb;
}

.docs-container {
  display: flex;
  max-width: 1200px;
  margin: 0 auto;
}

.docs-sidebar {
  width: 240px;
  background: white;
  border-right: 1px solid #e5e7eb;
  padding: 24px 16px;
  position: sticky;
  top: 0;
  height: 100vh;
  overflow-y: auto;
}

.docs-sidebar h3 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 16px;
  padding: 0 8px;
}

.docs-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  padding: 8px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  color: #6b7280;
  transition: all 0.2s;
}

.nav-item:hover {
  background: #f3f4f6;
}

.nav-item.active {
  background: #eff6ff;
  color: #3b82f6;
  font-weight: 500;
}

.docs-content {
  flex: 1;
  padding: 40px 48px;
}

.docs-section h1 {
  font-size: 32px;
  font-weight: 700;
  margin-bottom: 8px;
}

.lead {
  font-size: 18px;
  color: #6b7280;
  margin-bottom: 32px;
}

.docs-section h3 {
  font-size: 20px;
  font-weight: 600;
  margin: 32px 0 16px;
}

.docs-section h4 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 4px;
}

.code-block {
  background: #1e1e1e;
  border-radius: 8px;
  margin: 16px 0;
  overflow: hidden;
}

.code-header {
  background: #2d2d2d;
  padding: 8px 16px;
  font-size: 12px;
  color: #9ca3af;
}

.code-block pre {
  padding: 16px;
  margin: 0;
  overflow-x: auto;
}

.code-block code {
  color: #e5e7eb;
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 13px;
  line-height: 1.6;
}

.model-list {
  display: grid;
  gap: 16px;
}

.model-card {
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 12px;
  padding: 20px;
}

.model-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.model-header h3 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.tier-badge {
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 500;
}

.tier-badge.pro {
  background: #fef3c7;
  color: #92400e;
}

.tier-badge.base {
  background: #dbeafe;
  color: #1e40af;
}

.tier-badge.free {
  background: #d1fae5;
  color: #065f46;
}

.model-desc {
  color: #6b7280;
  margin-bottom: 12px;
}

.model-specs {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}

.spec {
  font-size: 13px;
  color: #6b7280;
}

.alert {
  padding: 16px;
  border-radius: 8px;
  margin: 16px 0;
}

.alert-info {
  background: #eff6ff;
  border-left: 3px solid #3b82f6;
  color: #1e40af;
}

.client-list {
  display: grid;
  gap: 12px;
}

.client-item {
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 16px;
}

.client-item p {
  color: #6b7280;
  margin: 0;
}
</style>
