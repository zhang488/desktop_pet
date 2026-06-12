import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import PenguinPet from './components/PenguinPet'
import './App.css'

type ReminderPayload = {
  kind: string
  name: string
  message: string
  timestamp: number
}

type Phase = 'idle' | 'appearing' | 'waiting' | 'leaving'

const THEME: Record<string, { color: string; icon: string }> = {
  drink_water: { color: '#3b82f6', icon: '💧' },
  eye_strain: { color: '#10b981', icon: '👀' },
  posture: { color: '#f59e0b', icon: '🧘' },
  lunch: { color: '#ef4444', icon: '🍱' },
  sleep: { color: '#8b5cf6', icon: '🌙' },
  manual: { color: '#6366f1', icon: '✨' },
}

function App() {
  const [phase, setPhase] = useState<Phase>('idle')
  const [reminder, setReminder] = useState<ReminderPayload | null>(null)
  const [mood, setMood] = useState<'normal' | 'happy'>('normal')

  useEffect(() => {
    let unlisten: UnlistenFn | undefined
    let disposed = false

    const show = (payload: ReminderPayload) => {
      setReminder(payload)
      setMood('normal')
      setPhase('appearing')
      window.setTimeout(() => setPhase('waiting'), 400)
    }

    listen<ReminderPayload>('reminder:trigger', (event) => {
      show(event.payload)
    }).then((fn) => {
      if (disposed) {
        fn()
        return
      }
      unlisten = fn
    })

    // 兜底：窗口刷新或 emit 事件丢失后，从后端拉回当前未响应的提醒
    invoke<ReminderPayload | null>('get_active_reminder')
      .then((payload) => {
        if (payload && !disposed) show(payload)
      })
      .catch((e) => console.error('get_active_reminder', e))

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [])

  const startDrag = async (e: React.MouseEvent) => {
    if (e.button !== 0) return
    if ((e.target as HTMLElement).closest('button')) return
    e.preventDefault()
    try {
      await getCurrentWindow().startDragging()
    } catch (err) {
      console.error('startDragging failed', err)
    }
  }

  const finalize = (mins?: number) => {
    const kind = reminder?.kind
    setPhase('leaving')
    window.setTimeout(async () => {
      if (mins == null) {
        await invoke('complete_reminder').catch((e) => console.error(e))
      } else if (mins === -1) {
        await invoke('skip_reminder').catch((e) => console.error(e))
      } else {
        await invoke('snooze_reminder', { kind, minutes: mins }).catch((e) => console.error(e))
      }
      setPhase('idle')
      setReminder(null)
      setMood('normal')
    }, 350)
  }

  const onComplete = () => {
    setMood('happy')
    finalize()
  }

  if (phase === 'idle') {
    return null
  }

  const theme = (reminder && THEME[reminder.kind]) || THEME.manual

  return (
    <div
      className={`pet-container phase-${phase}`}
      onMouseDown={startDrag}
      style={{ '--accent': theme.color } as React.CSSProperties}
    >
      <div className="pet-bubble">
        <span className="pet-bubble-icon">{theme.icon}</span>
        <span className="pet-bubble-text">{reminder?.message ?? '提醒来啦～'}</span>
      </div>
      <PenguinPet mood={mood} />
      <div className="pet-actions">
        <button className="btn btn-primary" onClick={onComplete}>搞定 ✓</button>
        <button className="btn btn-ghost" onClick={() => finalize(-1)}>跳过</button>
      </div>
      <div className="pet-snooze">
        <span className="snooze-label">小睡：</span>
        <button className="snooze-btn" onClick={() => finalize(5)}>5min</button>
        <button className="snooze-btn" onClick={() => finalize(10)}>10min</button>
        <button className="snooze-btn" onClick={() => finalize(15)}>15min</button>
      </div>
    </div>
  )
}

export default App
