import { createSignal, onMount, onCleanup } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { GameEntry } from '../shared/types'

const [games, setGames] = createSignal<GameEntry[]>([])
const [loading, setLoading] = createSignal(true)

async function fetchGames() {
  try {
    const result = await invoke<GameEntry[]>('load_games')
    setGames(result)
  } catch (err) {
    console.error('load_games failed:', err)
  } finally {
    setLoading(false)
  }
}

// Subscribe to push updates from the Rust backend. Returns an unlisten fn so
// callers can clean up when the component tree unmounts.
function subscribeToUpdates(): Promise<UnlistenFn> {
  return listen('gamesUpdated', () => {
    void fetchGames()
  })
}

// Convenience hook: fetch on mount and auto-subscribe.
function useGames() {
  onMount(() => {
    void fetchGames()
    let unlisten: UnlistenFn | undefined
    subscribeToUpdates().then((fn) => {
      unlisten = fn
    })
    onCleanup(() => unlisten?.())
  })

  return { games, loading }
}

export { games, loading, fetchGames, subscribeToUpdates, useGames }
