<template>
  <div class="docs-page">
    <div class="docs-container">
      <!-- Sidebar -->
      <aside class="docs-sidebar">
        <h3>Claude API 文档</h3>
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
          <p class="lead">使用您的 API Key 快速接入 Claude 模型</p>

          <div class="alert alert-info">
            <strong>获取 API Key：</strong>请先访问 <a href="/client" target="_blank">客户端页面</a> 注册/登录，系统会自动生成您的 API Key。
          </div>

          <h2>一、环境变量配置</h2>

          <!-- Linux/macOS -->
          <h3>🐧 Linux / macOS</h3>
          <div class="code-block">
            <div class="code-header">终端配置</div>
            <pre><code># 临时设置（当前终端会话有效）
export ANTHROPIC_API_KEY="sk-xxxxxx"
export ANTHROPIC_BASE_URL="https://115.190.62.87/v1"

# 永久设置（添加到 ~/.bashrc 或 ~/.zshrc）
echo 'export ANTHROPIC_API_KEY="sk-xxxxxx"' >> ~/.bashrc
echo 'export ANTHROPIC_BASE_URL="https://115.190.62.87/v1"' >> ~/.bashrc
source ~/.bashrc</code></pre>
          </div>

          <!-- Windows -->
          <h3>🪟 Windows (CMD / PowerShell)</h3>
          <div class="code-block">
            <div class="code-header">PowerShell 配置</div>
            <pre><code># 临时设置（当前会话有效）
$env:ANTHROPIC_API_KEY="sk-xxxxxx"
$env:ANTHROPIC_BASE_URL="https://115.190.62.87/v1"

# 永久设置（系统环境变量）
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-xxxxxx', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'https://115.190.62.87/v1', 'User')</code></pre>
          </div>

          <h2>二、Python SDK 安装与使用</h2>

          <h3>📦 安装依赖</h3>
          <div class="code-block">
            <div class="code-header">安装 anthropic 包</div>
            <pre><code># Linux / macOS
pip3 install anthropic

# Windows
pip install anthropic</code></pre>
          </div>

          <h3>🐍 Python 示例代码</h3>
          <div class="code-block">
            <div class="code-header">test_claude.py</div>
            <pre><code>from anthropic import Anthropic

# 初始化客户端
client = Anthropic(
    api_key="sk-xxxxxx",  # 替换为你的 API Key
    base_url="https://115.190.62.87/v1"
)

# 发送请求
message = client.messages.create(
    model="sonnet",
    max_tokens=1024,
    messages=[
        {"role": "user", "content": "你好，请介绍一下你自己。"}
    ]
)

# 输出响应
print(message.content[0].text)</code></pre>
          </div>

          <h2>三、Node.js SDK 安装与使用</h2>

          <h3>📦 安装依赖</h3>
          <div class="code-block">
            <div class="code-header">安装 @anthropic-ai/sdk</div>
            <pre><code># 初始化项目（如果还没有）
npm init -y

# 安装 SDK
npm install @anthropic-ai/sdk</code></pre>
          </div>

          <h3>📜 Node.js 示例代码</h3>
          <div class="code-block">
            <div class="code-header">test_claude.js</div>
            <pre><code>import Anthropic from '@anthropic-ai/sdk'

// 初始化客户端
const client = new Anthropic({
  apiKey: 'sk-xxxxxx',  // 替换为你的 API Key
  baseURL: 'https://115.190.62.87/v1'
})

// 发送请求
async function chat() {
  const message = await client.messages.create({
    model: 'sonnet',
    maxTokens: 1024,
    messages: [
      { role: 'user', content: '你好，请介绍一下你自己。' }
    ]
  })

  console.log(message.content[0].text)
}

chat()</code></pre>
          </div>

          <h2>四、curl 命令行测试</h2>

          <div class="code-block">
            <div class="code-header">curl 请求示例</div>
            <pre><code>curl https://115.190.62.87/v1/messages \
  -H "x-api-key: sk-xxxxxx" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "sonnet",
    "max_tokens": 1024,
    "messages": [
      {"role": "user", "content": "Hello, Claude!"}
    ]
  }'</code></pre>
          </div>
        </section>

        <!-- Models -->
        <section v-if="activeSection === 'models'" class="docs-section">
          <h1>支持的模型</h1>
          <p class="lead">我们提供 Claude 系列主流模型的中转服务</p>

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
              <div class="model-code">
                <code>{{ model.modelId }}</code>
              </div>
            </div>
          </div>
        </section>

        <!-- Installation -->
        <section v-if="activeSection === 'install'" class="docs-section">
          <h1>详细安装教程</h1>

          <!-- Linux -->
          <h2>🐧 Linux 安装教程</h2>

          <h3>1. 安装 Python 环境</h3>
          <div class="code-block">
            <pre><code># Ubuntu/Debian
sudo apt update
sudo apt install python3 python3-pip

# CentOS/RHEL/Fedora
sudo dnf install python3 python3-pip

# 验证安装
python3 --version
pip3 --version</code></pre>
          </div>

          <h3>2. 安装 Anthropic SDK</h3>
          <div class="code-block">
            <pre><code>pip3 install anthropic</code></pre>
          </div>

          <h3>3. 配置环境变量</h3>
          <div class="code-block">
            <pre><code># 编辑 ~/.bashrc
nano ~/.bashrc

# 添加以下内容
export ANTHROPIC_API_KEY="你的API_KEY"
export ANTHROPIC_BASE_URL="https://115.190.62.87/v1"

# 保存后执行
source ~/.bashrc</code></pre>
          </div>

          <h3>4. 创建测试文件</h3>
          <div class="code-block">
            <pre><code>cat > test_claude.py << 'EOF'
from anthropic import Anthropic
import os

client = Anthropic(
    api_key=os.environ.get("ANTHROPIC_API_KEY"),
    base_url=os.environ.get("ANTHROPIC_BASE_URL")
)

response = client.messages.create(
    model="sonnet",
    max_tokens=100,
    messages=[{"role": "user", "content": "Hi!"}]
)

print(response.content[0].text)
EOF

# 运行测试
python3 test_claude.py</code></pre>
          </div>

          <!-- macOS -->
          <h2>🍎 macOS 安装教程</h2>

          <h3>1. 安装 Homebrew（如果没有）</h3>
          <div class="code-block">
            <pre><code>/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"</code></pre>
          </div>

          <h3>2. 安装 Python</h3>
          <div class="code-block">
            <pre><code>brew install python3</code></pre>
          </div>

          <h3>3. 安装 Anthropic SDK</h3>
          <div class="code-block">
            <pre><code>pip3 install anthropic</code></pre>
          </div>

          <h3>4. 配置环境变量（zsh）</h3>
          <div class="code-block">
            <pre><code># 编辑 ~/.zshrc（macOS 默认使用 zsh）
nano ~/.zshrc

# 添加以下内容
export ANTHROPIC_API_KEY="你的API_KEY"
export ANTHROPIC_BASE_URL="https://115.190.62.87/v1"

# 保存后执行
source ~/.zshrc</code></pre>
          </div>

          <!-- Windows -->
          <h2>🪟 Windows 安装教程</h2>

          <h3>1. 安装 Python</h3>
          <div class="alert alert-warning">
            访问 <a href="https://python.org/downloads/" target="_blank">python.org/downloads</a> 下载并安装 Python 3.9+，安装时勾选 "Add Python to PATH"。
          </div>

          <h3>2. 打开 PowerShell</h3>
          <div class="code-block">
            <pre><code># 以管理员身份运行 PowerShell
# 按 Win+X，选择 "Windows PowerShell (管理员)" 或 "终端(管理员)"</code></pre>
          </div>

          <h3>3. 安装 Anthropic SDK</h3>
          <div class="code-block">
            <pre><code># 安装 anthropic 包
pip install anthropic

# 如果 pip 不存在，使用
python -m pip install anthropic</code></pre>
          </div>

          <h3>4. 设置环境变量（PowerShell）</h3>
          <div class="code-block">
            <pre><code># 临时设置（仅当前会话）
$env:ANTHROPIC_API_KEY="你的API_KEY"
$env:ANTHROPIC_BASE_URL="https://115.190.62.87/v1"

# 永久设置
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', '你的API_KEY', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'https://115.190.62.87/v1', 'User')

# 或者使用 setx 命令（CMD）
setx ANTHROPIC_API_KEY "你的API_KEY"
setx ANTHROPIC_BASE_URL "https://115.190.62.87/v1"</code></pre>
          </div>

          <h3>5. 创建测试文件</h3>
          <div class="code-block">
            <pre><code># 创建 test_claude.py 文件
@"
from anthropic import Anthropic
import os

client = Anthropic(
    api_key=os.environ.get("ANTHROPIC_API_KEY"),
    base_url=os.environ.get("ANTHROPIC_BASE_URL", "https://115.190.62.87/v1")
)

response = client.messages.create(
    model="sonnet",
    max_tokens=100,
    messages=[{"role": "user", "content": "Hi!"}]
)

print(response.content[0].text)
"@ | Out-File -Encoding UTF8 test_claude.py

# 运行测试
python test_claude.py</code></pre>
          </div>

          <h3>6. 图形界面设置环境变量（可选）</h3>
          <div class="alert alert-info">
            <ol>
              <li>按 Win+R，输入 <code>sysdm.cpl</code>，回车</li>
              <li>点击"高级"选项卡 → "环境变量"</li>
              <li>在"用户变量"中点击"新建"</li>
              <li>变量名: <code>ANTHROPIC_API_KEY</code>，变量值: 你的API Key</li>
              <li>再新建一个，变量名: <code>ANTHROPIC_BASE_URL</code>，变量值: <code>https://115.190.62.87/v1</code></li>
              <li>确定并重启终端</li>
            </ol>
          </div>
        </section>

        <!-- IDE Setup -->
        <section v-if="activeSection === 'ide'" class="docs-section">
          <h1>IDE 配置教程</h1>

          <!-- VS Code -->
          <h2>💻 VS Code 配置</h2>

          <div class="alert alert-info">
            VS Code 有多种方式接入 Claude API，推荐使用 Continue 插件。
          </div>

          <h3>方法一：Continue 插件（推荐）</h3>

          <h4>步骤 1：安装插件</h4>
          <ol class="step-list">
            <li>打开 VS Code，按 <code>Ctrl+Shift+X</code> 打开扩展商店</li>
            <li>搜索 <code>Continue</code> 并点击安装</li>
            <li>安装完成后点击右侧的 Continue 图标打开侧边栏</li>
          </ol>

          <h4>步骤 2：配置 API</h4>

          <div class="tabs-simple">
            <div :class="['tab-simple', { active: platformTab === 'windows' }]" @click="platformTab = 'windows'">Windows</div>
            <div :class="['tab-simple', { active: platformTab === 'linux' }]" @click="platformTab = 'linux'">Linux/macOS</div>
          </div>

          <template v-if="platformTab === 'windows'">
            <p><strong>Windows 配置文件位置：</strong></p>
            <div class="code-block">
              <div class="code-header">%USERPROFILE%\.continue\config.json</div>
              <pre><code>{
  "models": [{
    "title": "Claude Sonnet 4.6",
    "provider": "anthropic",
    "model": "sonnet",
    "apiKey": "sk-xxxxxx",
    "apiBase": "https://115.190.62.87/v1"
  }]
}</code></pre>
            </div>

            <h4>步骤 3：创建配置文件（Windows PowerShell）</h4>
            <div class="code-block">
              <pre><code># 创建配置目录
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.continue"

# 创建配置文件
@"
{
  "models": [{
    "title": "Claude Sonnet 4.6",
    "provider": "anthropic",
    "model": "sonnet",
    "apiKey": "你的API_KEY",
    "apiBase": "https://115.190.62.87/v1"
  }]
}
"@ | Out-File -Encoding UTF8 "$env:USERPROFILE\.continue\config.json"</code></pre>
            </div>
          </template>

          <template v-if="platformTab === 'linux'">
            <p><strong>Linux/macOS 配置文件位置：</strong></p>
            <div class="code-block">
              <div class="code-header">~/.continue/config.json</div>
              <pre><code>{
  "models": [{
    "title": "Claude Sonnet 4.6",
    "provider": "anthropic",
    "model": "sonnet",
    "apiKey": "sk-xxxxxx",
    "apiBase": "https://115.190.62.87/v1"
  }]
}</code></pre>
            </div>

            <h4>步骤 3：创建配置文件（Linux/macOS）</h4>
            <div class="code-block">
              <pre><code># 创建配置目录
mkdir -p ~/.continue

# 创建配置文件
cat > ~/.continue/config.json << 'EOF'
{
  "models": [{
    "title": "Claude Sonnet 4.6",
    "provider": "anthropic",
    "model": "sonnet",
    "apiKey": "你的API_KEY",
    "apiBase": "https://115.190.62.87/v1"
  }]
}
EOF</code></pre>
            </div>
          </template>

          <h4>步骤 4：验证配置</h4>
          <ol class="step-list">
            <li>重启 VS Code</li>
            <li>点击 Continue 侧边栏</li>
            <li>在聊天框输入 "你好" 测试</li>
            <li>如果能正常回复，配置成功</li>
          </ol>

          <h3>方法二：Roo Code 插件</h3>

          <h4>安装步骤</h4>
          <ol class="step-list">
            <li>在扩展商店搜索 <code>Roo Code</code></li>
            <li>安装后点击设置图标</li>
            <li>选择 "Custom OpenAI-compatible API"</li>
          </ol>

          <h4>配置参数</h4>
          <div class="code-block">
            <pre><code>API Base: https://115.190.62.87/v1
API Key: sk-xxxxxx
Model: sonnet</code></pre>
          </div>

          <h3>方法三：Cline 插件（原名 Claude Dev）</h3>

          <h4>安装步骤</h4>
          <ol class="step-list">
            <li>在扩展商店搜索 <code>Cline</code></li>
            <li>安装后点击 API 设置</li>
            <li>选择 "Anthropic-Compatible API"</li>
          </ol>

          <h4>配置参数</h4>
          <div class="code-block">
            <pre><code>API Base: https://115.190.62.87/v1
API Key: sk-xxxxxx
Model ID: sonnet</code></pre>
          </div>

          <h3>方法四：CodeGPT 插件</h3>

          <h4>安装步骤</h4>
          <ol class="step-list">
            <li>在扩展商店搜索 <code>CodeGPT</code></li>
            <li>安装后打开设置</li>
            <li>找到 "Provider" 设置</li>
            <li>选择 "Anthropic" 或 "Custom"</li>
          </ol>

          <h4>配置参数</h4>
          <div class="code-block">
            <pre><code>API Key: sk-xxxxxx
Base URL: https://115.190.62.87/v1
Model: sonnet</code></pre>
          </div>

          <h3>方法五：直接使用 .env 文件</h3>

          <p>某些插件支持通过项目根目录的 <code>.env</code> 文件配置：</p>
          <div class="code-block">
            <div class="code-header">.env</div>
            <pre><code>ANTHROPIC_API_KEY=sk-xxxxxx
ANTHROPIC_BASE_URL=https://115.190.62.87/v1</code></pre>
          </div>

          <!-- Cursor -->
          <h2>🎯 Cursor 配置</h2>

          <h3>方法一：图形界面配置（推荐）</h3>

          <ol class="step-list">
            <li>按 <code>Ctrl+,</code> (Windows/Linux) 或 <code>Cmd+,</code> (macOS) 打开设置</li>
            <li>在搜索框输入 <code>API Base</code></li>
            <li>填写以下信息：</li>
          </ol>

          <div class="code-block">
            <pre><code>API Base URL: https://115.190.62.87/v1
API Key: sk-xxxxxx
Model: sonnet</code></pre>
          </div>

          <h3>方法二：配置文件</h3>

          <p>编辑 <code>settings.json</code> 文件：</p>

          <div class="code-block">
            <div class="code-header">Windows: %APPDATA%\Cursor\User\settings.json</div>
            <div class="code-header">Linux/macOS: ~/.cursor/settings.json</div>
            <pre><code>{
  "cursor.api.apiBase": "https://115.190.62.87/v1",
  "cursor.api.apiKey": "sk-xxxxxx",
  "cursor.api.model": "sonnet"
}</code></pre>
          </div>

          <!-- Windsurf -->
          <h2>🌊 Windsurf 配置</h2>

          <ol class="step-list">
            <li>打开 Windsurf 设置（点击齿轮图标）</li>
            <li>找到 "AI Provider" 或 "模型设置"</li>
            <li>选择 "Custom Anthropic-compatible API"</li>
            <li>填写配置：</li>
          </ol>

          <div class="code-block">
            <pre><code>Base URL: https://115.190.62.87/v1
API Key: sk-xxxxxx
Model: sonnet</code></pre>
          </div>

          <!-- JetBrains -->
          <h2>🛩️ JetBrains 系列（PyCharm/IntelliJ IDEA）</h2>

          <h3>使用 CodeGPT 插件</h3>

          <ol class="step-list">
            <li>打开 <code>File</code> → <code>Settings</code> → <code>Plugins</code></li>
            <li>搜索并安装 <code>CodeGPT</code></li>
            <li>重启 IDE 后，打开 CodeGPT 设置</li>
            <li>选择 "Custom Provider" 或 "Anthropic"</li>
          </ol>

          <div class="code-block">
            <pre><code>Provider: Anthropic
API Key: sk-xxxxxx
Base URL: https://115.190.62.87/v1
Model: sonnet</code></pre>
          </div>

          <!-- Troubleshooting -->
          <h2>🔧 故障排除</h2>

          <h3>常见问题</h3>

          <div class="faq-item">
            <h4>问题：提示 "API Key 无效"</h4>
            <p>解决方案：请检查 API Key 是否正确，登录 <a href="/client" target="_blank">客户端</a> 获取最新的 Key。</p>
          </div>

          <div class="faq-item">
            <h4>问题：提示 "网络错误" 或 "请求超时"</h4>
            <p>解决方案：请检查网络连接，确认 Base URL 设置为 <code>https://115.190.62.87/v1</code></p>
          </div>

          <div class="faq-item">
            <h4>问题：配置文件不生效</h4>
            <p>解决方案：请确保 JSON 格式正确，重启 VS Code 使配置生效。</p>
          </div>

          <h3>配置验证测试</h3>

          <div class="code-block">
            <div class="code-header">在终端运行以下命令测试 API 是否可用</div>
            <pre><code>curl https://115.190.62.87/v1/messages \
  -H "x-api-key: 你的API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "sonnet",
    "max_tokens": 50,
    "messages": [{"role": "user", "content": "Hi"}]
  }'</code></pre>
          </div>

          <p>如果返回 JSON 响应且包含 <code>content</code> 字段，说明 API 配置正确。</p>
        </section>

        <!-- Authentication -->
        <section v-if="activeSection === 'auth'" class="docs-section">
          <h1>认证方式</h1>
          <p class="lead">API 通过请求头中的 x-api-key 进行认证</p>

          <div class="alert alert-info">
            您的 API Key 格式为 <code>sk-xxxxxx</code>，请在 <a href="/client" target="_blank">客户端页面</a> 登录后获取。
          </div>

          <h2>请求头格式</h2>
          <div class="code-block">
            <pre><code>x-api-key: sk-xxxxxx
anthropic-version: 2023-06-01
content-type: application/json</code></pre>
          </div>

          <h2>Python 认证示例</h2>
          <div class="code-block">
            <pre><code>from anthropic import Anthropic

# 方式一：直接传入
client = Anthropic(api_key="sk-xxxxxx", base_url="https://115.190.62.87/v1")

# 方式二：使用环境变量（推荐）
import os
client = Anthropic(
    api_key=os.environ.get("ANTHROPIC_API_KEY"),
    base_url=os.environ.get("ANTHROPIC_BASE_URL")
)</code></pre>
          </div>

          <h2>安全建议</h2>
          <ul class="feature-list">
            <li>不要在客户端代码中硬编码 API Key</li>
            <li>使用环境变量存储 API Key</li>
            <li>不要将 API Key 提交到 Git 仓库</li>
            <li>定期轮换 API Key</li>
            <li>监控 API 使用情况，发现异常及时处理</li>
          </ul>
        </section>

        <!-- Error Codes -->
        <section v-if="activeSection === 'errors'" class="docs-section">
          <h1>错误码</h1>
          <p class="lead">了解常见的错误码和解决方案</p>

          <table class="error-table">
            <thead>
              <tr>
                <th>状态码</th>
                <th>错误类型</th>
                <th>描述</th>
                <th>解决方案</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="error in errors" :key="error.code">
                <td><code>{{ error.code }}</code></td>
                <td>{{ error.type }}</td>
                <td>{{ error.description }}</td>
                <td>{{ error.solution }}</td>
              </tr>
            </tbody>
          </table>
        </section>

        <!-- FAQ -->
        <section v-if="activeSection === 'faq'" class="docs-section">
          <h1>常见问题</h1>

          <div class="faq-item" v-for="(faq, index) in faqs" :key="index">
            <h3>{{ faq.question }}</h3>
            <div v-html="faq.answer"></div>
          </div>
        </section>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const activeSection = ref('quickstart')
const platformTab = ref('windows')

const sections = [
  { id: 'quickstart', title: '🚀 快速开始' },
  { id: 'models', title: '🤖 模型列表' },
  { id: 'install', title: '📦 安装教程' },
  { id: 'ide', title: '💻 IDE 配置' },
  { id: 'auth', title: '🔑 认证说明' },
  { id: 'errors', title: '⚠️ 错误码' },
  { id: 'faq', title: '❓ 常见问题' }
]

const models = [
  {
    id: 'opus',
    name: 'Opus',
    modelId: 'opus',
    tier: 'pro',
    description: '最强性能模型，适合复杂任务',
    context: '200K',
    inputPrice: '15.0',
    outputPrice: '75.0'
  },
  {
    id: 'sonnet',
    name: 'Sonnet',
    modelId: 'sonnet',
    tier: 'base',
    description: '平衡性能和成本，推荐使用',
    context: '200K',
    inputPrice: '3.0',
    outputPrice: '15.0'
  },
  {
    id: 'haiku',
    name: 'Haiku',
    modelId: 'haiku',
    tier: 'free',
    description: '快速响应模型，适合简单任务',
    context: '200K',
    inputPrice: '0.25',
    outputPrice: '1.25'
  }
]

const errors = [
  {
    code: '401',
    type: 'Unauthorized',
    description: 'API Key 无效或已过期',
    solution: '请检查 API Key 是否正确，登录客户端获取最新的 Key'
  },
  {
    code: '402',
    type: 'Payment Required',
    description: '余额不足',
    solution: '请使用充值卡充值，或联系管理员'
  },
  {
    code: '429',
    type: 'Rate Limit',
    description: '请求过于频繁',
    solution: '请降低请求频率，基础用户支持 20 并发'
  },
  {
    code: '400',
    type: 'Bad Request',
    description: '请求参数错误',
    solution: '请检查请求格式，确保 model、messages 等参数正确'
  },
  {
    code: '500',
    type: 'Internal Error',
    description: '服务器内部错误',
    solution: '请联系管理员或稍后重试'
  }
]

const faqs = [
  {
    question: '如何获取 API Key？',
    answer: '访问 <a href="/client" target="_blank">客户端页面</a>，注册/登录后，系统会自动生成您的 API Key。'
  },
  {
    question: '支持哪些工具和平台？',
    answer: '支持所有兼容 Anthropic/Claude API 的工具，包括 Continue、Cursor、Windsurf、LangChain、LlamaIndex 等。'
  },
  {
    question: '请求有并发限制吗？',
    answer: '基础用户支持 20 并发请求，专业版用户支持更高并发。'
  },
  {
    question: '如何充值？',
    answer: '在 <a href="/client" target="_blank">客户端页面</a> 使用充值卡充值，联系管理员获取充值卡。'
  },
  {
    question: '支持流式输出吗？',
    answer: '支持，在请求中设置 <code>stream=True</code> (Python) 或 <code>stream: true</code> (JavaScript) 即可。'
  },
  {
    question: '模型名称如何填写？',
    answer: '使用以下模型 ID：<br>• Opus: <code>opus</code><br>• Sonnet: <code>sonnet</code><br>• Haiku: <code>haiku</code>'
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
  max-width: 1400px;
  margin: 0 auto;
}

.docs-sidebar {
  width: 260px;
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
  padding: 10px 12px;
  border-radius: 8px;
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
  max-width: 900px;
}

.docs-section h1 {
  font-size: 32px;
  font-weight: 700;
  margin-bottom: 8px;
}

.docs-section h2 {
  font-size: 24px;
  font-weight: 600;
  margin: 40px 0 16px;
  padding-bottom: 8px;
  border-bottom: 1px solid #e5e7eb;
}

.docs-section h3 {
  font-size: 18px;
  font-weight: 600;
  margin: 24px 0 12px;
}

.lead {
  font-size: 18px;
  color: #6b7280;
  margin-bottom: 32px;
}

.code-block {
  background: #1e1e1e;
  border-radius: 8px;
  margin: 12px 0;
  overflow: hidden;
}

.code-header {
  background: #2d2d2d;
  padding: 8px 16px;
  font-size: 12px;
  color: #9ca3af;
  font-weight: 500;
}

.code-block pre {
  padding: 16px;
  margin: 0;
  overflow-x: auto;
}

.code-block code {
  color: #e5e7eb;
  font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
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
  transition: all 0.2s;
}

.model-card:hover {
  box-shadow: 0 4px 12px rgba(0,0,0,0.1);
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

.model-code {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid #f0f0f0;
}

.model-code code {
  background: #f3f4f6;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-family: monospace;
  color: #667eea;
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

.alert-warning {
  background: #fffbeb;
  border-left: 3px solid #f59e0b;
  color: #92400e;
}

.alert code {
  background: rgba(255,255,255,0.5);
  padding: 2px 6px;
  border-radius: 4px;
}

.alert a {
  color: inherit;
  text-decoration: underline;
  font-weight: 500;
}

.feature-list {
  list-style: none;
  padding: 0;
}

.feature-list li {
  padding: 8px 0;
  padding-left: 24px;
  position: relative;
}

.feature-list li:before {
  content: "✓";
  position: absolute;
  left: 0;
  color: #10b981;
  font-weight: bold;
}

.step-list {
  padding-left: 20px;
}

.step-list li {
  margin-bottom: 8px;
  line-height: 1.6;
}

.error-table {
  width: 100%;
  border-collapse: collapse;
  margin: 16px 0;
}

.error-table th,
.error-table td {
  text-align: left;
  padding: 12px;
  border-bottom: 1px solid #e5e7eb;
}

.error-table th {
  background: #f9fafb;
  font-weight: 600;
}

.error-table code {
  background: #f3f4f6;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}

.faq-item {
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 16px;
}

.faq-item h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 8px;
}

.faq-item p {
  color: #6b7280;
  margin: 0;
}

/* 响应式 */
@media (max-width: 768px) {
  .docs-container {
    flex-direction: column;
  }

  .docs-sidebar {
    width: 100%;
    height: auto;
    position: relative;
  }

  .docs-content {
    padding: 24px;
  }
}

/* 新增样式 */
.docs-section h4 {
  font-size: 16px;
  font-weight: 600;
  margin: 20px 0 10px;
  color: #374151;
}

.tabs-simple {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
  border-bottom: 1px solid #e5e7eb;
}

.tab-simple {
  padding: 8px 16px;
  border-radius: 6px 6px 0 0;
  font-size: 13px;
  cursor: pointer;
  color: #6b7280;
  border: 1px solid transparent;
}

.tab-simple.active {
  background: white;
  color: #3b82f6;
  border-color: #e5e7eb #e5e7eb white #e5e7eb;
}

</style>
