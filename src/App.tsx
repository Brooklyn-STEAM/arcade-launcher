import { createSignal, createEffect, For, Show, onMount, onCleanup } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { useGames } from './stores/gameStore'
import { focusedIndex, setFocusedIndex, startInputLoop, stopInputLoop } from './stores/inputStore'
import { GameDetail } from './components/GameDetail'
import type { GameEntry } from './shared/types'
import './App.css'

// ---------------------------------------------------------------------------
// GameTile
// ---------------------------------------------------------------------------

interface GameTileProps {
  game: GameEntry
  index: number
  focused: boolean
  onClick: () => void
}

function GameTile(props: GameTileProps) {
  let tileRef: HTMLDivElement | undefined = undefined

  // Scroll the focused tile into view when focus lands on it via gamepad
  createEffect(() => {
    if (props.focused) {
      tileRef?.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' })
    }
  })

  return (
    <div
      ref={(el) => {
        tileRef = el
      }}
      class="game-tile"
      classList={{ focused: props.focused }}
      tabIndex={-1}
      onClick={() => props.onClick()}
    >
      <div class="tile-thumb">
        <Show
          when={props.game.thumbnailPath}
          fallback={<span class="tile-thumb-placeholder">{props.game.title.charAt(0)}</span>}
        >
          <img src={`http://localhost:8037/${props.game.thumbnailPath}`} alt={props.game.title} />
        </Show>
      </div>
      <div class="tile-info">
        <p class="tile-title">{props.game.title}</p>
        <p class="tile-author">{props.game.author}</p>
        <p class="tile-version">v{props.game.version}</p>
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

function App() {
  const { games, loading } = useGames()
  const [selectedGame, setSelectedGame] = createSignal<GameEntry | null>(null)
  const [launchError, setLaunchError] = createSignal<string | null>(null)

  // The grid element ref — used to compute column count for up/down navigation
  let gridRef: HTMLDivElement | undefined = undefined

  // Compute current number of columns by measuring the grid's computed style
  function getColCount(): number {
    if (!gridRef) return 4
    const style = window.getComputedStyle(gridRef)
    const cols = style.getPropertyValue('grid-template-columns').split(' ').length
    return cols > 0 ? cols : 4
  }

  const enabledGames = () => games().filter((g) => g.enabled)

  // Confirm action: dismiss error popup first; otherwise open/launch
  function handleConfirm() {
    if (launchError()) {
      dismissError()
      return
    }
    const detail = selectedGame()
    if (detail) {
      // Detail is open — launch the game (close first, then invoke)
      handleLaunch(detail)
    } else {
      const game = enabledGames()[focusedIndex()]
      if (game) setSelectedGame(game)
    }
  }

  // Back action: dismiss error popup first, then close detail if open
  function handleBack() {
    if (launchError()) {
      dismissError()
      return
    }
    setSelectedGame(null)
  }

  // Launch: close detail screen then invoke launch_game
  function handleLaunch(game: GameEntry) {
    setSelectedGame(null)
    invoke('launch_game', { gameId: game.id }).catch((err: unknown) => {
      const msg = typeof err === 'string' ? err : 'Failed to launch game.'
      setLaunchError(msg)
    })
  }

  function dismissError() {
    setLaunchError(null)
  }

  // Keyboard fallback
  function handleKeyDown(e: KeyboardEvent) {
    // Don't interfere with form inputs
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return

    // Error popup takes priority — any key dismisses it
    if (launchError()) {
      e.preventDefault()
      dismissError()
      return
    }

    const detail = selectedGame()

    if (detail) {
      if (e.key === 'Escape') {
        e.preventDefault()
        setSelectedGame(null)
      } else if (e.key === 'Enter') {
        e.preventDefault()
        handleLaunch(detail)
      }
      return
    }

    const total = enabledGames().length
    if (total === 0) return
    const cols = getColCount()

    switch (e.key) {
      case 'ArrowLeft':
        e.preventDefault()
        setFocusedIndex((i) => (i > 0 ? i - 1 : i))
        break
      case 'ArrowRight':
        e.preventDefault()
        setFocusedIndex((i) => (i < total - 1 ? i + 1 : i))
        break
      case 'ArrowUp':
        e.preventDefault()
        setFocusedIndex((i) => (i - cols >= 0 ? i - cols : i))
        break
      case 'ArrowDown':
        e.preventDefault()
        setFocusedIndex((i) => (i + cols < total ? i + cols : i))
        break
      case 'Enter':
        e.preventDefault()
        handleConfirm()
        break
      case 'Escape':
        e.preventDefault()
        handleBack()
        break
    }
  }

  onMount(() => {
    document.addEventListener('keydown', handleKeyDown)

    startInputLoop(() => enabledGames().length, getColCount, handleConfirm, handleBack)
  })

  onCleanup(() => {
    document.removeEventListener('keydown', handleKeyDown)
    stopInputLoop()
  })

  // Clamp focusedIndex when the game list shrinks
  createEffect(() => {
    const total = enabledGames().length
    if (total > 0 && focusedIndex() >= total) {
      setFocusedIndex(total - 1)
    }
  })

  return (
    <main class="shell">
      <header class="header">
        <span class="header-title">ARCADE</span>
        <span class="header-hint">
          Admin UI: <code>http://localhost:8037</code>
        </span>
      </header>

      <Show when={loading()}>
        <div class="center-message">
          <span class="loading-dots">Loading</span>
        </div>
      </Show>

      <Show when={!loading() && enabledGames().length === 0}>
        <div class="center-message">
          <p>NO GAMES LOADED</p>
          <p class="hint">Open the admin UI to add a game:</p>
          <code>http://localhost:8037</code>
        </div>
      </Show>

      <Show when={!loading() && enabledGames().length > 0}>
        <div
          class="game-grid"
          ref={(el) => {
            gridRef = el
          }}
        >
          <For each={enabledGames()}>
            {(game, index) => (
              <GameTile
                game={game}
                index={index()}
                focused={focusedIndex() === index()}
                onClick={() => {
                  setFocusedIndex(index())
                  setSelectedGame(game)
                }}
              />
            )}
          </For>
        </div>
      </Show>

      <Show when={selectedGame() !== null}>
        <GameDetail
          game={selectedGame()!}
          onClose={handleBack}
          onLaunch={() => handleLaunch(selectedGame()!)}
        />
      </Show>

      <Show when={launchError() !== null}>
        <div class="error-overlay" onClick={() => dismissError()}>
          <div class="error-dialog" onClick={(e) => e.stopPropagation()}>
            <p class="error-title">LAUNCH FAILED</p>
            <p class="error-message">{launchError()}</p>
            <button class="error-btn" onClick={() => dismissError()}>
              OK
            </button>
          </div>
        </div>
      </Show>
    </main>
  )
}

export default App
