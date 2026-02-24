<template>
  <div class="admin-users">
    <div class="page-header">
      <h1>用户管理</h1>
      <button @click="showAddModal = true" class="btn btn-primary">
        添加用户
      </button>
    </div>

    <!-- Filters -->
    <div class="filters">
      <input
        v-model="searchQuery"
        type="text"
        placeholder="搜索用户名或邮箱"
        class="input"
      />
      <select v-model="tierFilter" class="input">
        <option value="">全部等级</option>
        <option value="pro">专业版</option>
        <option value="base">基础版</option>
        <option value="free">免费版</option>
      </select>
    </div>

    <!-- Users Table -->
    <div class="table-container">
      <table class="users-table">
        <thead>
          <tr>
            <th>用户ID</th>
            <th>用户名</th>
            <th>邮箱</th>
            <th>等级</th>
            <th>余额</th>
            <th>注册时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="user in filteredUsers" :key="user.user_id">
            <td>{{ user.user_id }}</td>
            <td>{{ user.username }}</td>
            <td>{{ user.email }}</td>
            <td>
              <span :class="['tier-badge', user.tier]">{{ tierLabels[user.tier] }}</span>
            </td>
            <td>{{ formatBalance(user.balance) }}</td>
            <td>{{ formatDate(user.created_at) }}</td>
            <td>
              <div class="actions">
                <button @click="editUser(user)" class="action-btn">编辑</button>
                <button @click="adjustBalance(user)" class="action-btn">充值</button>
                <button @click="viewHistory(user)" class="action-btn">历史</button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    <div class="pagination">
      <button :disabled="currentPage === 1" @click="currentPage--">上一页</button>
      <span>第 {{ currentPage }} 页</span>
      <button :disabled="!hasMore" @click="currentPage++">下一页</button>
    </div>

    <!-- Edit Modal -->
    <Modal v-model:show="showEditModal" title="编辑用户">
      <div v-if="selectedUser" class="form-group">
        <label>用户名</label>
        <input v-model="selectedUser.username" class="input" />
      </div>
      <div v-if="selectedUser" class="form-group">
        <label>邮箱</label>
        <input v-model="selectedUser.email" class="input" />
      </div>
      <div v-if="selectedUser" class="form-group">
        <label>等级</label>
        <select v-model="selectedUser.tier" class="input">
          <option value="pro">专业版</option>
          <option value="base">基础版</option>
          <option value="free">免费版</option>
        </select>
      </div>
      <template #footer>
        <button @click="showEditModal = false" class="btn">取消</button>
        <button @click="saveUser" class="btn btn-primary">保存</button>
      </template>
    </Modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import apiClient from '@/api/client'
import Modal from '@/components/common/Modal.vue'

interface User {
  user_id: string
  username: string
  email: string
  tier: string
  balance: number
  created_at: string
}

const users = ref<User[]>([])
const searchQuery = ref('')
const tierFilter = ref('')
const currentPage = ref(1)
const pageSize = 20
const hasMore = ref(false)

const showEditModal = ref(false)
const showAddModal = ref(false)
const selectedUser = ref<User | null>(null)

const tierLabels: Record<string, string> = {
  pro: '专业版',
  base: '基础版',
  free: '免费版'
}

const filteredUsers = computed(() => {
  let result = users.value

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(u =>
      u.username.toLowerCase().includes(query) ||
      u.email.toLowerCase().includes(query)
    )
  }

  if (tierFilter.value) {
    result = result.filter(u => u.tier === tierFilter.value)
  }

  const start = (currentPage.value - 1) * pageSize
  const end = start + pageSize
  hasMore.value = end < result.length

  return result.slice(start, end)
})

function formatBalance(num: number): string {
  if (num >= 10000) {
    return (num / 10000).toFixed(1) + '万'
  }
  return num.toLocaleString()
}

function formatDate(date: string): string {
  return new Date(date).toLocaleDateString('zh-CN')
}

async function loadUsers() {
  try {
    const response = await apiClient.get('/admin/users')
    users.value = response.users || []
  } catch (error) {
    console.error('Failed to load users:', error)
  }
}

function editUser(user: User) {
  selectedUser.value = { ...user }
  showEditModal.value = true
}

async function saveUser() {
  if (!selectedUser.value) return

  try {
    await apiClient.post('/admin/users/update', selectedUser.value)
    showEditModal.value = false
    loadUsers()
  } catch (error) {
    console.error('Failed to save user:', error)
  }
}

function adjustBalance(user: User) {
  const amount = prompt('输入充值金额（正数增加，负数扣除）:')
  if (amount) {
    apiClient.post('/admin/users/adjust-balance', {
      user_id: user.user_id,
      amount: parseInt(amount)
    }).then(() => {
      loadUsers()
    })
  }
}

function viewHistory(user: User) {
  // Navigate to user history
  console.log('View history for:', user.user_id)
}

onMounted(() => {
  loadUsers()
})
</script>

<style scoped>
.admin-users {
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

.filters {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
}

.filters .input {
  flex: 1;
  max-width: 300px;
}

.table-container {
  background: white;
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}

.users-table {
  width: 100%;
  border-collapse: collapse;
}

.users-table th,
.users-table td {
  padding: 16px;
  text-align: left;
  border-bottom: 1px solid #e5e7eb;
}

.users-table th {
  background: #f9fafb;
  font-weight: 600;
  font-size: 13px;
  color: #6b7280;
}

.users-table td {
  font-size: 14px;
}

.tier-badge {
  display: inline-block;
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
</style>
