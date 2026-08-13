<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  getAdminProfiles,
  createSharedBook,
  getShares,
  upsertShare,
  removeShare,
  getUsers,
  getGroups,
  type ShareItem,
} from '../api/addressBook'

interface BookItem {
  guid: string
  name: string
  owner: string
  rule: number
  is_personal: boolean
}

const books = ref<BookItem[]>([])
const sharesMap = ref<Record<string, ShareItem[]>>({})
const users = ref<{ id: number; username: string }[]>([])
const groups = ref<{ id: number; name: string }[]>([])
const loading = ref(false)
const error = ref('')
const notice = ref('')

const newBookName = ref('')
const shareForms = ref<Record<string, { targetType: 'user' | 'group'; targetId: string; rule: number }>>({})

async function load() {
  loading.value = true
  error.value = ''
  try {
    const [booksRes, usersRes, groupsRes] = await Promise.all([
      getAdminProfiles(),
      getUsers(),
      getGroups(),
    ])
    books.value = booksRes.data.filter((b: BookItem) => !b.is_personal)
    users.value = usersRes.data
    groups.value = groupsRes.data
    for (const b of books.value) {
      const s = await getShares(b.guid)
      sharesMap.value[b.guid] = s.data
      shareForms.value[b.guid] = { targetType: 'user', targetId: '', rule: 2 }
    }
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function handleCreate() {
  error.value = ''
  if (!newBookName.value.trim()) {
    error.value = 'Book name is required'
    return
  }
  try {
    await createSharedBook(newBookName.value.trim())
    newBookName.value = ''
    notice.value = 'Shared book created'
    await load()
  } catch (e: any) {
    error.value = e.message
  }
}

async function handleShare(book: BookItem) {
  error.value = ''
  const form = shareForms.value[book.guid]
  if (!form.targetId) {
    error.value = 'Choose a user or group to share with'
    return
  }
  try {
    await upsertShare({
      ab_guid: book.guid,
      user_id: form.targetType === 'user' ? Number(form.targetId) : undefined,
      group_id: form.targetType === 'group' ? Number(form.targetId) : undefined,
      rule: form.rule,
    })
    notice.value = `Shared "${book.name}"`
    await load()
  } catch (e: any) {
    error.value = e.message
  }
}

async function handleRemoveShare(book: BookItem, share: ShareItem) {
  error.value = ''
  try {
    await removeShare({
      ab_guid: book.guid,
      user_id: share.kind === 'user' ? Number(share.target) : undefined,
      group_id: share.kind === 'group' ? Number(share.target) : undefined,
    })
    notice.value = `Share removed from "${book.name}"`
    await load()
  } catch (e: any) {
    error.value = e.message
  }
}

const ruleLabel = (r: number) => (r === 1 ? 'Read' : r === 2 ? 'Read-write' : 'Admin')

onMounted(load)
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-lg font-semibold text-gray-900 dark:text-rd-text">Shared Books</h1>
        <p class="text-sm text-gray-500 dark:text-rd-text-secondary mt-1">
          Shared address books with per-user / per-group permissions.
        </p>
      </div>
      <button
        @click="load"
        class="px-4 py-2 text-sm rounded-lg bg-rd-primary hover:bg-rd-primary-hover text-white font-medium transition-colors"
      >
        Refresh
      </button>
    </div>

    <div v-if="error" class="mb-4 p-3 text-sm rounded-lg bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400">
      {{ error }}
    </div>
    <div v-if="notice" class="mb-4 p-3 text-sm rounded-lg bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400">
      {{ notice }}
    </div>

    <!-- Create -->
    <div class="mb-6 p-4 bg-white dark:bg-rd-card rounded-xl border border-gray-200 dark:border-rd-border flex gap-3">
      <input
        v-model="newBookName"
        placeholder="New shared book name"
        class="flex-1 px-3 py-2 text-sm rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-card-hover text-gray-900 dark:text-rd-text focus:outline-none focus:ring-2 focus:ring-rd-primary"
        @keyup.enter="handleCreate"
      />
      <button
        @click="handleCreate"
        class="px-4 py-2 text-sm rounded-lg bg-rd-primary hover:bg-rd-primary-hover text-white font-medium transition-colors"
      >
        Create
      </button>
    </div>

    <!-- Books -->
    <div v-for="book in books" :key="book.guid" class="mb-6 p-4 bg-white dark:bg-rd-card rounded-xl border border-gray-200 dark:border-rd-border">
      <div class="flex items-center justify-between mb-3">
        <div>
          <span class="font-semibold text-gray-900 dark:text-rd-text">{{ book.name }}</span>
          <span class="ml-2 text-xs text-gray-500 dark:text-rd-text-secondary">owner: {{ book.owner }}</span>
        </div>
      </div>

      <!-- Shares -->
      <div class="mb-3 space-y-2">
        <div
          v-for="share in sharesMap[book.guid] || []"
          :key="share.ab_guid + share.target + share.kind"
          class="flex items-center justify-between px-3 py-2 rounded-lg bg-gray-50 dark:bg-rd-card-hover text-sm"
        >
          <span class="text-gray-700 dark:text-rd-text">
            {{ share.kind === 'user' ? '👤' : '👥' }} {{ share.target }}
            <span class="ml-2 text-xs px-2 py-0.5 rounded-full bg-rd-primary/10 text-rd-primary">{{ ruleLabel(share.rule) }}</span>
          </span>
          <button
            @click="handleRemoveShare(book, share)"
            class="text-xs text-red-500 hover:text-red-700"
          >
            Remove
          </button>
        </div>
        <div v-if="!(sharesMap[book.guid] || []).length" class="text-xs text-gray-400 dark:text-rd-text-secondary">
          Not shared yet.
        </div>
      </div>

      <!-- Add share -->
      <div class="flex gap-2 items-center">
        <select
          v-model="shareForms[book.guid].targetType"
          class="px-2 py-1.5 text-sm rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-card-hover text-gray-900 dark:text-rd-text"
        >
          <option value="user">User</option>
          <option value="group">Group</option>
        </select>
        <select
          v-model="shareForms[book.guid].targetId"
          class="flex-1 px-2 py-1.5 text-sm rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-card-hover text-gray-900 dark:text-rd-text"
        >
          <option value="" disabled>Select…</option>
          <option v-if="shareForms[book.guid].targetType === 'user'" v-for="u in users" :key="u.id" :value="String(u.id)">
            {{ u.username }}
          </option>
          <option v-else v-for="g in groups" :key="g.id" :value="String(g.id)">
            {{ g.name }}
          </option>
        </select>
        <select
          v-model="shareForms[book.guid].rule"
          class="px-2 py-1.5 text-sm rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-card-hover text-gray-900 dark:text-rd-text"
        >
          <option :value="1">Read</option>
          <option :value="2">Read-write</option>
          <option :value="3">Admin</option>
        </select>
        <button
          @click="handleShare(book)"
          class="px-3 py-1.5 text-sm rounded-lg border border-rd-primary text-rd-primary hover:bg-rd-primary hover:text-white transition-colors"
        >
          Share
        </button>
      </div>
    </div>

    <div v-if="!loading && books.length === 0" class="text-center text-sm text-gray-500 dark:text-rd-text-secondary py-8">
      No shared books yet — create one above.
    </div>
  </div>
</template>
