import { api } from './client'

export interface Peer {
  id: string
  hash: string
  username: string
  hostname: string
  platform: string
  alias: string
  tags: string[]
  note: string
}

export interface Tag {
  name: string
  color: number
}

export interface AbProfile {
  guid: string
  name: string
  owner: string
  rule: number
  note: string
}

export interface PeersResponse {
  data: Peer[]
  total: number
}

export interface TagsResponse {
  data: Tag[]
  total: number
}

export function getPersonalAb(): Promise<{ data: AbProfile }> {
  return api('/api/ab/personal')
}

export function getPeers(guid: string, page = 1, pageSize = 100): Promise<PeersResponse> {
  return api(`/api/ab/peers?ab=${guid}&current=${page}&pageSize=${pageSize}`)
}

export function addPeer(guid: string, peer: Partial<Peer>): Promise<void> {
  return api(`/api/ab/peer/add/${guid}`, {
    method: 'POST',
    body: JSON.stringify(peer),
  })
}

export function deletePeer(guid: string, ids: string[]): Promise<void> {
  return api(`/api/ab/peer/${guid}`, {
    method: 'DELETE',
    body: JSON.stringify({ ids }),
  })
}

export function getTags(guid: string): Promise<TagsResponse> {
  return api(`/api/ab/tags/${guid}`)
}

export function addTag(guid: string, name: string, color: number): Promise<void> {
  return api(`/api/ab/tag/add/${guid}`, {
    method: 'POST',
    body: JSON.stringify({ name, color }),
  })
}

export function renameTag(guid: string, oldName: string, newName: string): Promise<void> {
  return api(`/api/ab/tag/rename/${guid}`, {
    method: 'PUT',
    body: JSON.stringify({ old: oldName, new: newName }),
  })
}

export function updateTagColor(guid: string, name: string, color: number): Promise<void> {
  return api(`/api/ab/tag/update/${guid}`, {
    method: 'PUT',
    body: JSON.stringify({ name, color }),
  })
}

export function deleteTag(guid: string, names: string[]): Promise<void> {
  return api(`/api/ab/tag/${guid}`, {
    method: 'DELETE',
    body: JSON.stringify({ names }),
  })
}

// Admin APIs
export interface UserItem {
  id: number
  username: string
  name: string
  email: string
  is_admin: boolean
  status: number
  created_at: string
}

export interface GroupItem {
  id: number
  name: string
  note: string
  created_at: string
}

export function getUsers(): Promise<{ data: UserItem[]; total: number }> {
  return api('/api/users')
}

export function createUser(user: { username: string; password: string; name?: string; email?: string; is_admin?: boolean }): Promise<void> {
  return api('/api/users', { method: 'POST', body: JSON.stringify(user) })
}

export function updateUser(id: number, data: Record<string, any>): Promise<void> {
  return api(`/api/users/${id}`, { method: 'PUT', body: JSON.stringify(data) })
}

export function deleteUser(id: number): Promise<void> {
  return api(`/api/users/${id}`, { method: 'DELETE' })
}

export function getGroups(): Promise<{ data: GroupItem[]; total: number }> {
  return api('/api/groups')
}

export function createGroup(group: { name: string; note?: string }): Promise<void> {
  return api('/api/groups', { method: 'POST', body: JSON.stringify(group) })
}

export function updateGroup(id: number, data: Record<string, any>): Promise<void> {
  return api(`/api/groups/${id}`, { method: 'PUT', body: JSON.stringify(data) })
}

export function deleteGroup(id: number): Promise<void> {
  return api(`/api/groups/${id}`, { method: 'DELETE' })
}

// ===== Shared books (dev branch) =====

export interface ShareItem {
  ab_guid: string
  target: string
  kind: 'user' | 'group'
  rule: number
}

export interface SharedProfilesResponse {
  data: AbProfile[]
}

export function createSharedBook(name: string): Promise<{ guid: string; name: string }> {
  return api('/api/ab/create', {
    method: 'POST',
    body: JSON.stringify({ name }),
  })
}

export function getSharedProfiles(): Promise<SharedProfilesResponse> {
  return api('/api/ab/shared/profiles')
}

export function getShares(guid: string): Promise<{ data: ShareItem[] }> {
  return api(`/api/ab/shares?ab=${guid}`)
}

export function upsertShare(share: { ab_guid: string; user_id?: number; group_id?: number; rule: number }): Promise<void> {
  return api('/api/ab/share', {
    method: 'POST',
    body: JSON.stringify(share),
  })
}

export function removeShare(share: { ab_guid: string; user_id?: number; group_id?: number }): Promise<void> {
  return api('/api/ab/share', {
    method: 'DELETE',
    body: JSON.stringify(share),
  })
}

export function getAdminProfiles(): Promise<{ data: { guid: string; name: string; owner: string; rule: number; is_personal: boolean }[] }> {
  return api('/api/ab/admin/profiles')
}
