import { api } from './client'

export interface Device {
  id: string
  hostname: string
  platform: string
  os: string
  version: string
  username: string
  cpu: string
  memory: string
  last_online: string
  online: boolean
}

export interface DevicesResponse {
  data: Device[]
  total: number
}

export function getDevices(): Promise<DevicesResponse> {
  return api('/api/devices')
}
