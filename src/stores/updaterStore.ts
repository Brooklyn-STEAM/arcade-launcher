import { createSignal, onMount } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'

export interface UpdateInfo {
  version: string
  body: string | null
}

const [updateInfo, setUpdateInfo] = createSignal<UpdateInfo | null>(null)
const [installing, setInstalling] = createSignal(false)
const [installError, setInstallError] = createSignal<string | null>(null)

async function checkForUpdate() {
  try {
    const info = await invoke<UpdateInfo | null>('check_for_update')
    setUpdateInfo(info)
  } catch (err) {
    // Silently swallow check errors (e.g. offline, dev build) — not fatal
    console.warn('update check failed:', err)
  }
}

async function installUpdate() {
  setInstalling(true)
  setInstallError(null)
  try {
    await invoke('install_update')
    // install_update restarts the app on success; if we get here something odd happened
  } catch (err) {
    const msg = typeof err === 'string' ? err : 'Update failed.'
    setInstallError(msg)
  } finally {
    setInstalling(false)
  }
}

function dismissUpdate() {
  setUpdateInfo(null)
  setInstallError(null)
}

/** Call inside any component that should trigger the update check on mount. */
function useUpdater() {
  onMount(() => {
    void checkForUpdate()
  })
  return { updateInfo, installing, installError, installUpdate, dismissUpdate }
}

export { updateInfo, installing, installError, checkForUpdate, installUpdate, dismissUpdate, useUpdater }
