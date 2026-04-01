import { Show } from 'solid-js'
import type { GameEntry } from '../shared/types'
import './GameDetail.css'

interface GameDetailProps {
  game: GameEntry | null
  onLaunch: () => void
}

export function GameDetail(props: GameDetailProps) {
  return (
    <div class="detail-sidebar">
      <Show
        when={props.game}
        fallback={
          <div class="detail-empty">
            <span class="detail-empty-hint">SELECT A GAME</span>
          </div>
        }
      >
        {(game) => (
          <>
            {/* Thumbnail */}
            <div class="detail-thumb-img">
              <Show
                when={game().thumbnailPath}
                fallback={<span class="detail-thumb-placeholder">{game().title.charAt(0)}</span>}
              >
                <img src={`http://localhost:8037/${game().thumbnailPath}`} alt={game().title} />
              </Show>
            </div>

            {/* Title strip */}
            <div class="detail-title-strip">
              <h2 class="detail-title">{game().title}</h2>
            </div>

            {/* Meta + description + actions */}
            <div class="detail-info-col">
              <div class="detail-meta">
                <p class="detail-author">{game().author}</p>
                <p class="detail-version">v{game().version}</p>
              </div>

              <Show when={game().description}>
                <p class="detail-description">{game().description}</p>
              </Show>

              <div class="detail-actions">
                <button class="detail-btn detail-btn-launch" onClick={() => props.onLaunch()}>
                  PRESS START
                </button>
              </div>
            </div>
          </>
        )}
      </Show>
    </div>
  )
}
