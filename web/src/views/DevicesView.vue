<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getDevices, type Device } from '../api/devices'
import { getPersonalAb, addPeer } from '../api/addressBook'

const devices = ref<Device[]>([])
const loading = ref(false)
const error = ref('')
const notice = ref('')

async function loadDevices() {
  loading.value = true
  error.value = ''
  try {
    const res = await getDevices()
    devices.value = res.data
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function addToBook(device: Device) {
  error.value = ''
  notice.value = ''
  try {
    const { data } = await getPersonalAb()
    await addPeer(data.guid, {
      id: device.id,
      hostname: device.hostname,
      username: device.username,
      platform: device.platform,
      alias: device.hostname,
    })
    notice.value = `Added ${device.hostname || device.id} to the address book`
  } catch (e: any) {
    error.value = e.message
  }
}

function timeAgo(ts: string): string {
  const t = new Date(ts.replace(' ', 'T') + 'Z').getTime()
  if (isNaN(t)) return ts
  const diff = Math.floor((Date.now() - t) / 1000)
  if (diff < 60) return 'just now'
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
  return `${Math.floor(diff / 86400)}d ago`
}

onMounted(loadDevices)
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-lg font-semibold text-gray-900 dark:text-rd-text">Devices</h1>
        <p class="text-sm text-gray-500 dark:text-rd-text-secondary mt-1">
          Workstations that registered via the client sysinfo/heartbeat (auto-discovery).
        </p>
      </div>
      <button
        @click="loadDevices"
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

    <div class="bg-white dark:bg-rd-card rounded-xl border border-gray-200 dark:border-rd-border overflow-hidden">
      <table class="w-full text-sm">
        <thead>
          <tr class="text-left text-xs uppercase tracking-wider text-gray-500 dark:text-rd-text-secondary border-b border-gray-200 dark:border-rd-border">
            <th class="px-4 py-3">Status</th>
            <th class="px-4 py-3">Hostname</th>
            <th class="px-4 py-3">RustDesk ID</th>
            <th class="px-4 py-3">Platform / OS</th>
            <th class="px-4 py-3">User</th>
            <th class="px-4 py-3">Last seen</th>
            <th class="px-4 py-3"></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="d in devices"
            :key="d.id"
            class="border-b border-gray-100 dark:border-rd-border last:border-0"
          >
            <td class="px-4 py-3">
              <span
                class="inline-flex items-center gap-1.5 text-xs font-medium"
                :class="d.online ? 'text-green-600 dark:text-green-400' : 'text-gray-400'"
              >
                <span class="w-2 h-2 rounded-full" :class="d.online ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"></span>
                {{ d.online ? 'Online' : 'Offline' }}
              </span>
            </td>
            <td class="px-4 py-3 font-medium text-gray-900 dark:text-rd-text">{{ d.hostname || '—' }}</td>
            <td class="px-4 py-3 font-mono text-xs text-gray-600 dark:text-rd-text-secondary">{{ d.id }}</td>
            <td class="px-4 py-3 text-gray-600 dark:text-rd-text-secondary">
              {{ d.platform || '—' }}{{ d.os ? ' · ' + d.os : '' }}
            </td>
            <td class="px-4 py-3 text-gray-600 dark:text-rd-text-secondary">{{ d.username || '—' }}</td>
            <td class="px-4 py-3 text-gray-500 dark:text-rd-text-secondary">{{ timeAgo(d.last_online) }}</td>
            <td class="px-4 py-3 text-right">
              <button
                @click="addToBook(d)"
                class="px-3 py-1.5 text-xs rounded-lg border border-rd-primary text-rd-primary hover:bg-rd-primary hover:text-white transition-colors"
              >
                Add to book
              </button>
            </td>
          </tr>
          <tr v-if="!loading && devices.length === 0">
            <td colspan="7" class="px-4 py-8 text-center text-sm text-gray-500 dark:text-rd-text-secondary">
              No devices registered yet. Devices appear here once the RustDesk client syncs
              (needs a working <code>/api/sysinfo</code> on the API server).
            </td>
          </tr>
          <tr v-if="loading">
            <td colspan="7" class="px-4 py-8 text-center text-sm text-gray-500">Loading…</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
