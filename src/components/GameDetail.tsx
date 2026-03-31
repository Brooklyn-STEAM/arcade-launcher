import { Show } from 'solid-js'
import type { GameEntry } from '../shared/types'
import './GameDetail.css'

interface GameDetailProps {
  game: GameEntry
  onClose: () => void
  onLaunch: () => void
}

export function GameDetail(props: GameDetailProps) {
  return (
    <div class="detail-overlay" onClick={() => props.onClose()}>
      {/* Stop click propagation from the panel itself so clicking inside doesn't dismiss */}
      <div class="detail-panel" onClick={(e) => e.stopPropagation()}>
        {/* Title header bar */}
        <div class="detail-header">
          <h2 class="detail-title">{props.game.title}</h2>
        </div>

        {/* Side-by-side body: thumbnail left, info right */}
        <div class="detail-body">
          {/* Left: thumbnail */}
          <div class="detail-thumb-col">
            <Show
              when={props.game.thumbnailPath}
              fallback={<span class="detail-thumb-placeholder">{props.game.title.charAt(0)}</span>}
            >
              <img
                src={`http://localhost:8037/${props.game.thumbnailPath}`}
                alt={props.game.title}
              />
            </Show>
          </div>

          {/* Right: meta + description + buttons */}
          <div class="detail-info-col">
            <div class="detail-meta">
              <p class="detail-author">{props.game.author}</p>
              <p class="detail-version">v{props.game.version}</p>
            </div>

            <Show when={props.game.description}>
              <p class="detail-description">{props.game.description}</p>
            </Show>

            <div class="detail-actions">
              <button class="detail-btn detail-btn-launch" onClick={() => props.onLaunch()}>
                START
              </button>
              <button class="detail-btn detail-btn-back" onClick={() => props.onClose()}>
                BACK
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
