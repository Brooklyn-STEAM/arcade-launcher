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
