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
      <div class="detail-panel" onClick={(e) => e.stopPropagation()}>
        <div class="detail-body">
          {/* Left: thumbnail + title strip below — mirrors the game tile layout */}
          <div class="detail-thumb-col">
            <div class="detail-thumb-img">
              <Show
                when={props.game.thumbnailPath}
                fallback={
                  <span class="detail-thumb-placeholder">{props.game.title.charAt(0)}</span>
                }
              >
                <img
                  src={`http://localhost:8037/${props.game.thumbnailPath}`}
                  alt={props.game.title}
                />
              </Show>
            </div>
            <div class="detail-title-strip">
              <h2 class="detail-title">{props.game.title}</h2>
            </div>
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
                PRESS START
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
