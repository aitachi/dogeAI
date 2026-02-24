<template>
  <div class="admin-cards">
    <div class="page-header">
      <h1>充值卡管理</h1>
      <button @click="showCreateModal = true" class="btn btn-primary">
        生成充值卡
      </button>
    </div>

    <!-- Stats -->
    <div class="stats-row">
      <div class="stat-item">
        <span class="stat-label">总卡数</span>
        <span class="stat-value">{{ stats.total }}</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">已使用</span>
        <span class="stat-value">{{ stats.used }}</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">未使用</span>
        <span class="stat-value">{{ stats.available }}</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">总金额</span>
        <span class="stat-value">¥{{ stats.totalAmount.toLocaleString() }}</span>
      </div>
    </div>

    <!-- Cards Table -->
    <div class="table-container">
      <table class="cards-table">
        <thead>
          <tr>
            <th>卡号</th>
            <th>面值</th>
            <th>状态</th>
            <th>使用用户</th>
            <th>使用时间</th>
            <th>创建时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="card in cards" :key="card.card_id">
            <td><code>{{ card.code }}</code></td>
            <td>¥{{ card.amount.toLocaleString() }}</td>
            <td>
              <span :class="['status-badge', card.is_used ? 'used' : 'available']">
                {{ card.is_used ? '已使用' : '可用' }}
              </span>
            </td>
            <td>{{ card.used_by || '-' }}</td>
            <td>{{ card.used_at ? formatDate(card.used_at) : '-' }}</td>
            <td>{{ formatDate(card.created_at) }}</td>
            <td>
              <div class="actions">
                <button @click="copyCard(card.code)" class="action-btn">复制</button>
                <button v-if="!card.is_used" @click="deleteCard(card.card_id)" class="action-btn danger">删除</button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    <div class="pagination">
      <button :disabled="currentPage === 1" @click="currentPage--">上一页</button>
      <span>第 {{ currentPage }} / {{ totalPages }} 页</span>
      <button :disabled="currentPage >= totalPages" @click="currentPage++">下一页</button>
    </div>

    <!-- Create Modal -->
    <Modal v-model:show="showCreateModal" title="生成充值卡">
      <div class="form-group">
        <label>面值</label>
        <input v-model.number="createForm.amount" type="number" class="input" placeholder="输入面值" />
      </div>
      <div class="form-group">
        <label>数量</label>
        <input v-model.number="createForm.count" type="number" class="input" placeholder="生成数量" />
      </div>
      <div class="form-group">
        <label>前缀</label>
        <input v-model="createForm.prefix" type="text" class="input" placeholder="卡号前缀（可选）" />
      </div>
      <template #footer>
        <button @click="showCreateModal = false" class="btn">取消</button>
        <button @click="createCards" class="btn btn-primary" :disabled="isCreating">
          {{ isCreating ? '生成中...' : '生成' }}
        </button>
      </template>
    </Modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import apiClient from '@/api/client'
import Modal from '@/components/common/Modal.vue'

interface RechargeCard {
  card_id: number
  code: string
  amount: number
  is_used: boolean
  used_by?: string
  used_at?: string
  created_at: string
}

const cards = ref<RechargeCard[]>([])
const currentPage = ref(1)
const pageSize = 20
const totalCards = ref(0)

const showCreateModal = ref(false)
const isCreating = ref(false)

const createForm = ref({
  amount: 1000,
  count: 10,
  prefix: ''
})

const stats = computed(() => {
  const used = cards.value.filter(c => c.is_used).length
  const totalAmount = cards.value.reduce((sum, c) => sum + c.amount, 0)
  return {
    total: cards.value.length,
    used,
    available: cards.value.length - used,
    totalAmount
  }
})

const totalPages = computed(() => Math.ceil(totalCards.value / pageSize))

function formatDate(date: string): string {
  return new Date(date).toLocaleString('zh-CN')
}

async function loadCards() {
  try {
    const response = await apiClient.get('/admin/recharge-cards', {
      params: { page: currentPage.value, limit: pageSize }
    })
    cards.value = response.cards || []
    totalCards.value = response.total || 0
  } catch (error) {
    console.error('Failed to load cards:', error)
  }
}

async function createCards() {
  isCreating.value = true
  try {
    await apiClient.post('/admin/recharge-cards/create', createForm.value)
    showCreateModal.value = false
    createForm.value = { amount: 1000, count: 10, prefix: '' }
    loadCards()
  } catch (error) {
    console.error('Failed to create cards:', error)
  }
  isCreating.value = false
}

async function deleteCard(cardId: number) {
  if (!confirm('确定要删除这张充值卡吗？')) return

  try {
    await apiClient.delete(`/admin/recharge-cards/${cardId}`)
    loadCards()
  } catch (error) {
    console.error('Failed to delete card:', error)
  }
}

function copyCard(code: string) {
  navigator.clipboard.writeText(code).then(() => {
    alert(`已复制: ${code}`)
  }).catch(() => {
    alert('复制失败，请手动复制')
  })
}

onMounted(() => {
  loadCards()
})
</script>

<style scoped>
.admin-cards {
  padding: 24px;
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.page-header h1 {
  font-size: 24px;
  font-weight: 700;
}

.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 24px;
}

.stat-item {
  background: white;
  padding: 20px;
  border-radius: 12px;
  text-align: center;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}

.stat-label {
  font-size: 13px;
  color: #6b7280;
}

.stat-value {
  font-size: 24px;
  font-weight: 700;
  color: #1f2937;
}

.table-container {
  background: white;
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}

.cards-table {
  width: 100%;
  border-collapse: collapse;
}

.cards-table th,
.cards-table td {
  padding: 16px;
  text-align: left;
  border-bottom: 1px solid #e5e7eb;
}

.cards-table th {
  background: #f9fafb;
  font-weight: 600;
  font-size: 13px;
  color: #6b7280;
}

.cards-table td {
  font-size: 14px;
}

.cards-table code {
  background: #f3f4f6;
  padding: 4px 8px;
  border-radius: 4px;
  font-family: monospace;
}

.status-badge {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 500;
}

.status-badge.available {
  background: #d1fae5;
  color: #065f46;
}

.status-badge.used {
  background: #fee2e2;
  color: #991b1b;
}

.actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  padding: 6px 12px;
  border-radius: 4px;
  border: 1px solid #e5e7eb;
  background: white;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn:hover {
  background: #f3f4f6;
}

.action-btn.danger {
  color: #ef4444;
  border-color: #fecaca;
}

.action-btn.danger:hover {
  background: #fef2f2;
}

.pagination {
  display: flex;
  justify-content: center;
  gap: 16px;
  align-items: center;
  margin-top: 24px;
}

.pagination button {
  padding: 8px 16px;
  border-radius: 6px;
  border: 1px solid #e5e7eb;
  background: white;
  cursor: pointer;
}

.pagination button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  margin-bottom: 8px;
  font-weight: 500;
  font-size: 14px;
}

.btn {
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border: none;
}

.btn-primary {
  background: linear-gradient(135deg, #667eea, #764ba2);
  color: white;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
