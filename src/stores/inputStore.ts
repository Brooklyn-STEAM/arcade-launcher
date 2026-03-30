/**
 * inputStore — Gamepad polling loop + focus navigation state.
 *
 * Exports:
 *   focusedIndex  — reactive signal: which game tile is currently focused
 *   setFocusedIndex
 *   startInputLoop(getGameCount, getColCount)
 *     — starts the rAF polling loop; call once from the root component.
 *       Pass reactive accessors so the loop always uses the latest values.
 *   stopInputLoop  — cancels the rAF loop (called on cleanup)
 */

import { createSignal } from 'solid-js'

// ---------------------------------------------------------------------------
// Deadzone + debounce helpers
// ---------------------------------------------------------------------------

const DEADZONE = 0.25

function applyDeadzone(value: number): number {
  return Math.abs(value) > DEADZONE ? value : 0
}

// We track the "pressed" state of every button from the previous frame so we
// only fire an action on the leading edge of a press (not while held).
let prevButtonStates: boolean[][] = []

function wasJustPressed(gamepadIndex: number, buttonIndex: number, pressed: boolean): boolean {
  const prev = prevButtonStates[gamepadIndex]?.[buttonIndex] ?? false
  return pressed && !prev
}

// ---------------------------------------------------------------------------
// Focus signal
// ---------------------------------------------------------------------------

const [focusedIndex, setFocusedIndex] = createSignal(0)

// ---------------------------------------------------------------------------
// Direction state for analog stick (fire once per tilt, not continuously)
// ---------------------------------------------------------------------------

type Direction = 'up' | 'down' | 'left' | 'right' | null
let prevStickDir: Direction[] = []

function stickDirection(x: number, y: number): Direction {
  const ax = applyDeadzone(x)
  const ay = applyDeadzone(y)
  if (ax === 0 && ay === 0) return null
  if (Math.abs(ay) >= Math.abs(ax)) {
    return ay < 0 ? 'up' : 'down'
  }
  return ax < 0 ? 'left' : 'right'
}

// ---------------------------------------------------------------------------
// Navigation logic
// ---------------------------------------------------------------------------

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value))
}

function navigate(
  dir: Direction,
  gameCount: number,
  colCount: number,
  onConfirm: () => void,
  onBack: () => void,
) {
  if (!dir) return
  setFocusedIndex((prev) => {
    const total = gameCount
    if (total === 0) return prev
    switch (dir) {
      case 'left':
        return prev > 0 ? prev - 1 : prev
      case 'right':
        return prev < total - 1 ? prev + 1 : prev
      case 'up': {
        const next = prev - colCount
        return next >= 0 ? next : prev
      }
      case 'down': {
        const next = prev + colCount
        return next < total ? next : prev
      }
      default:
        return prev
    }
  })
  void onConfirm // referenced to satisfy linter; confirm/back handled via button events
  void onBack
}

// ---------------------------------------------------------------------------
// rAF loop
// ---------------------------------------------------------------------------

let rafId: number | null = null

export function startInputLoop(
  getGameCount: () => number,
  getColCount: () => number,
  onConfirm: () => void,
  onBack: () => void,
) {
  function loop() {
    const gamepads = navigator.getGamepads()

    for (let gi = 0; gi < gamepads.length; gi++) {
      const gp = gamepads[gi]
      if (!gp) continue

      // Ensure previous-state arrays are initialised for this gamepad
      if (!prevButtonStates[gi]) {
        prevButtonStates[gi] = new Array(gp.buttons.length).fill(false)
      }
      if (!prevStickDir[gi]) {
        prevStickDir[gi] = null
      }

      const gameCount = getGameCount()
      const colCount = getColCount()

      // --- D-pad buttons (indices 12–15: up, down, left, right) ---
      const dpadMap: [number, Direction][] = [
        [12, 'up'],
        [13, 'down'],
        [14, 'left'],
        [15, 'right'],
      ]
      for (const [idx, dir] of dpadMap) {
        const pressed = gp.buttons[idx]?.pressed ?? false
        if (wasJustPressed(gi, idx, pressed)) {
          navigate(dir, gameCount, colCount, onConfirm, onBack)
        }
      }

      // --- Analog left stick ---
      const stickX = gp.axes[0] ?? 0
      const stickY = gp.axes[1] ?? 0
      const currentDir = stickDirection(stickX, stickY)
      if (currentDir !== prevStickDir[gi] && currentDir !== null) {
        navigate(currentDir, gameCount, colCount, onConfirm, onBack)
      }
      prevStickDir[gi] = currentDir

      // --- A (0) / Start (9) → confirm ---
      const aPressed = gp.buttons[0]?.pressed ?? false
      const startPressed = gp.buttons[9]?.pressed ?? false
      if (wasJustPressed(gi, 0, aPressed) || wasJustPressed(gi, 9, startPressed)) {
        onConfirm()
      }

      // --- B (1) → back ---
      const bPressed = gp.buttons[1]?.pressed ?? false
      if (wasJustPressed(gi, 1, bPressed)) {
        onBack()
      }

      // --- LB (4) → page left, RB (5) → page right ---
      const lbPressed = gp.buttons[4]?.pressed ?? false
      const rbPressed = gp.buttons[5]?.pressed ?? false
      if (wasJustPressed(gi, 4, lbPressed)) {
        setFocusedIndex((prev) => clamp(prev - colCount, 0, getGameCount() - 1))
      }
      if (wasJustPressed(gi, 5, rbPressed)) {
        setFocusedIndex((prev) => clamp(prev + colCount, 0, getGameCount() - 1))
      }

      // Update previous button states for this gamepad
      prevButtonStates[gi] = gp.buttons.map((b) => b.pressed)
    }

    rafId = requestAnimationFrame(loop)
  }

  rafId = requestAnimationFrame(loop)
}

export function stopInputLoop() {
  if (rafId !== null) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
  prevButtonStates = []
  prevStickDir = []
}

export { focusedIndex, setFocusedIndex }
