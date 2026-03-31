import { createEffect, Show } from 'solid-js'
import type { GameEntry } from '../shared/types'

export interface GameTileProps {
  game: GameEntry
  index: number
  focused: boolean
  onClick: () => void
}

export function GameTile(props: GameTileProps) {
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
      classList={{ focused: props.focused, 'tile-disabled': !props.game.enabled }}
      tabIndex={-1}
      onClick={() => props.onClick()}
    >
      {/* Red title band at top — NES box art style */}
      <div class="tile-header">
        <span class="tile-title-text">{props.game.title}</span>
        <Show when={!props.game.enabled}>
          <span class="tile-badge-off">OFF</span>
        </Show>
      </div>

      {/* Thumbnail */}
      <div class="tile-thumb">
        <Show
          when={props.game.thumbnailPath}
          fallback={<span class="tile-thumb-placeholder">{props.game.title.charAt(0)}</span>}
        >
          <img src={`http://localhost:8037/${props.game.thumbnailPath}`} alt={props.game.title} />
        </Show>
      </div>

      {/* Footer strip — author + version */}
      <div class="tile-footer">
        <span class="tile-author">By: {props.game.author}</span>
        <span class="tile-version">v{props.game.version}</span>
      </div>
    </div>
  )
}
