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
      {/* DISABLED badge — top-left */}
      <Show when={!props.game.enabled}>
        <span class="tile-badge-off">OFF</span>
      </Show>

      {/* Thumbnail — clean, no overlay */}
      <div class="tile-thumb">
        <Show
          when={props.game.thumbnailPath}
          fallback={<span class="tile-thumb-placeholder">{props.game.title.charAt(0)}</span>}
        >
          <img src={`http://localhost:8037/${props.game.thumbnailPath}`} alt={props.game.title} />
        </Show>
      </div>

      {/* Title strip below the thumbnail */}
      <div class="tile-info">
        <span class="tile-title-text">{props.game.title}</span>
      </div>
    </div>
  )
}
