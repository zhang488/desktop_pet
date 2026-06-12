import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './Settings.css'

type TaskInfo = {
  id: string
  name: string
  enabled: boolean
  kind: 'interval' | 'daily'
  interval_secs: number | null
  daily_hour: number | null
  daily_minute: number | null
}

type DndConfig = {
  enabled: boolean
  start_hour: number
  start_minute: number
  end_hour: number
  end_minute: number
}

type GlobalSettings = {
  paused: boolean
  dnd: DndConfig
  autostart: boolean
}

type TodayCount = {
  kind: string
  triggered: number
  completed: number
}

type DailyPoint = {
  date: string
  triggered: number
  completed: number
}

type Stats = {
  today: TodayCount[]
  last_days: DailyPoint[]
  streak_days: number
  total_completed: number
}

const KIND_THEME: Record<string, { color: string; icon: string }> = {
  drink_water: { color: '#3b82f6', icon: '💧' },
  eye_strain: { color: '#10b981', icon: '👀' },
  posture: { color: '#f59e0b', icon: '🧘' },
  lunch: { color: '#ef4444', icon: '🍱' },
  sleep: { color: '#8b5cf6', icon: '🌙' },
}

function pad2(n: number) {
  return n.toString().padStart(2, '0')
}

function timeToHM(value: string): [number, number] | null {
  const [h, m] = value.split(':').map((v) => parseInt(v, 10))
  if (!Number.isFinite(h) || !Number.isFinite(m)) return null
  return [h, m]
}

function Settings() {
  const [tasks, setTasks] = useState<TaskInfo[]>([])
  const [global, setGlobal] = useState<GlobalSettings | null>(null)
  const [stats, setStats] = useState<Stats | null>(null)
  const [loading, setLoading] = useState(true)

  const reload = useCallback(async () => {
    try {
      const [t, g, s] = await Promise.all([
        invoke<TaskInfo[]>('list_tasks'),
        invoke<GlobalSettings>('get_global_settings'),
        invoke<Stats>('get_stats', { days: 7 }),
      ])
      setTasks(t)
      setGlobal(g)
      setStats(s)
    } catch (e) {
      console.error('reload', e)
    } finally {
      setLoading(false)
    }
  }, [])

  const reloadStats = useCallback(async () => {
    try {
      const s = await invoke<Stats>('get_stats', { days: 7 })
      setStats(s)
    } catch (e) {
      console.error(e)
    }
  }, [])

  useEffect(() => {
    const id = window.setInterval(reloadStats, 15000)
    return () => window.clearInterval(id)
  }, [reloadStats])

  useEffect(() => {
    reload()
  }, [reload])

  const patchTask = (id: string, patch: Partial<TaskInfo>) =>
    setTasks((prev) => prev.map((t) => (t.id === id ? { ...t, ...patch } : t)))

  const patchGlobal = (patch: Partial<GlobalSettings>) =>
    setGlobal((g) => (g ? { ...g, ...patch } : g))

  const patchDnd = (patch: Partial<DndConfig>) =>
    setGlobal((g) => (g ? { ...g, dnd: { ...g.dnd, ...patch } } : g))

  const onToggle = async (id: string, enabled: boolean) => {
    patchTask(id, { enabled })
    await invoke('set_task_enabled', { id, enabled }).catch((e) => console.error(e))
  }

  const onIntervalChange = async (id: string, intervalSecs: number) => {
    if (!Number.isFinite(intervalSecs) || intervalSecs < 5) return
    patchTask(id, { interval_secs: intervalSecs })
    await invoke('set_task_interval', { id, intervalSecs }).catch((e) => console.error(e))
  }

  const onDailyChange = async (id: string, hour: number, minute: number) => {
    patchTask(id, { daily_hour: hour, daily_minute: minute })
    await invoke('set_task_daily', { id, hour, minute }).catch((e) => console.error(e))
  }

  const onTriggerNow = async (id: string) =>
    invoke('trigger_now', { kind: id }).catch((e) => console.error(e))

  const onTogglePaused = async (paused: boolean) => {
    patchGlobal({ paused })
    await invoke('set_paused', { paused }).catch((e) => console.error(e))
  }

  const onToggleAutostart = async (enabled: boolean) => {
    patchGlobal({ autostart: enabled })
    await invoke('set_autostart', { enabled }).catch((e) => {
      console.error(e)
      patchGlobal({ autostart: !enabled })
    })
  }

  const persistDnd = async (next: DndConfig) => {
    await invoke('set_dnd', {
      enabled: next.enabled,
      startHour: next.start_hour,
      startMinute: next.start_minute,
      endHour: next.end_hour,
      endMinute: next.end_minute,
    }).catch((e) => console.error(e))
  }

  const onToggleDnd = async (enabled: boolean) => {
    if (!global) return
    const next = { ...global.dnd, enabled }
    patchDnd({ enabled })
    await persistDnd(next)
  }

  const onDndStartChange = async (value: string) => {
    const hm = timeToHM(value)
    if (!hm || !global) return
    const next = { ...global.dnd, start_hour: hm[0], start_minute: hm[1] }
    patchDnd({ start_hour: hm[0], start_minute: hm[1] })
    await persistDnd(next)
  }

  const onDndEndChange = async (value: string) => {
    const hm = timeToHM(value)
    if (!hm || !global) return
    const next = { ...global.dnd, end_hour: hm[0], end_minute: hm[1] }
    patchDnd({ end_hour: hm[0], end_minute: hm[1] })
    await persistDnd(next)
  }

  if (loading || !global) {
    return <div className="settings-loading">加载中…</div>
  }

  return (
    <div className="settings-app">
      <header className="settings-header">
        <h1>提醒设置</h1>
        <p className="settings-sub">配置桌宠的各项定时提醒</p>
      </header>

      <main className="settings-list">
        {/* 统计卡片 */}
        {stats && <StatsCard stats={stats} />}

        {/* 全局设置区 */}
        <section className="task-card global-card">
          <div className="global-head">
            <span className="task-icon">⚙️</span>
            <span className="task-name">全局</span>
          </div>

          <div className="row">
            <label>暂停全部</label>
            <div className="control">
              <Switch checked={global.paused} onChange={onTogglePaused} />
              <span className="hint">{global.paused ? '所有自动提醒已暂停' : '提醒正常运行'}</span>
            </div>
          </div>

          <div className="row">
            <label>开机自启</label>
            <div className="control">
              <Switch checked={global.autostart} onChange={onToggleAutostart} />
              <span className="hint">系统启动时自动运行桌宠</span>
            </div>
          </div>

          <div className="row">
            <label>勿扰</label>
            <div className="control">
              <Switch checked={global.dnd.enabled} onChange={onToggleDnd} />
              <span className="hint">设定时段内不弹出自动提醒</span>
            </div>
          </div>

          {global.dnd.enabled && (
            <div className="row dnd-times">
              <label>时段</label>
              <div className="control">
                <input
                  type="time"
                  value={`${pad2(global.dnd.start_hour)}:${pad2(global.dnd.start_minute)}`}
                  onChange={(e) => onDndStartChange(e.target.value)}
                />
                <span className="dash">—</span>
                <input
                  type="time"
                  value={`${pad2(global.dnd.end_hour)}:${pad2(global.dnd.end_minute)}`}
                  onChange={(e) => onDndEndChange(e.target.value)}
                />
                <span className="hint">支持跨天，例如 23:00 — 07:00</span>
              </div>
            </div>
          )}
        </section>

        {/* 各提醒任务 */}
        {tasks.map((t) => {
          const theme = KIND_THEME[t.id] ?? { color: '#6366f1', icon: '✨' }
          return (
            <section
              key={t.id}
              className={`task-card ${t.enabled ? '' : 'disabled'}`}
              style={{ borderLeftColor: theme.color }}
            >
              <div className="task-head">
                <span className="task-icon">{theme.icon}</span>
                <span className="task-name">{t.name}</span>
                <Switch checked={t.enabled} onChange={(v) => onToggle(t.id, v)} />
              </div>

              <div className="task-body">
                {t.kind === 'interval' ? (
                  <div className="row">
                    <label>间隔</label>
                    <div className="control">
                      <input
                        type="number"
                        min={5}
                        step={5}
                        value={t.interval_secs ?? 60}
                        onChange={(e) => onIntervalChange(t.id, parseInt(e.target.value, 10) || 0)}
                        disabled={!t.enabled}
                      />
                      <span className="unit">秒</span>
                      <span className="hint">
                        ≈ {((t.interval_secs ?? 60) / 60).toFixed(1)} 分钟
                      </span>
                    </div>
                  </div>
                ) : (
                  <div className="row">
                    <label>每天</label>
                    <div className="control">
                      <input
                        type="time"
                        value={`${pad2(t.daily_hour ?? 12)}:${pad2(t.daily_minute ?? 0)}`}
                        onChange={(e) => {
                          const hm = timeToHM(e.target.value)
                          if (hm) onDailyChange(t.id, hm[0], hm[1])
                        }}
                        disabled={!t.enabled}
                      />
                    </div>
                  </div>
                )}

                <div className="row actions">
                  <button className="btn-test" onClick={() => onTriggerNow(t.id)}>
                    立即测试 →
                  </button>
                </div>
              </div>
            </section>
          )
        })}
      </main>

      <footer className="settings-footer">
        <span>修改即时生效 · 配置保存在 %APPDATA%/Desktop Pet/</span>
      </footer>
    </div>
  )
}

function Switch({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="switch">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span className="slider" />
    </label>
  )
}

function StatsCard({ stats }: { stats: Stats }) {
  const todayTriggered = stats.today.reduce((s, c) => s + c.triggered, 0)
  const todayCompleted = stats.today.reduce((s, c) => s + c.completed, 0)
  const todayRate = todayTriggered === 0 ? 0 : Math.round((todayCompleted / todayTriggered) * 100)

  const max = Math.max(1, ...stats.last_days.map((d) => d.triggered))
  const weekdayNames = ['日', '一', '二', '三', '四', '五', '六']

  return (
    <section className="stats-card">
      <div className="stats-head">
        <span className="task-icon">📊</span>
        <span className="task-name">打卡数据</span>
        <span className="stats-total">累计 {stats.total_completed} 次</span>
      </div>

      <div className="stats-numbers">
        <div className="stat-item">
          <div className="stat-value">{stats.streak_days}</div>
          <div className="stat-label">连续天数</div>
        </div>
        <div className="stat-item">
          <div className="stat-value">{todayCompleted}<span className="stat-suffix">/{todayTriggered}</span></div>
          <div className="stat-label">今日完成</div>
        </div>
        <div className="stat-item">
          <div className="stat-value">{todayRate}<span className="stat-suffix">%</span></div>
          <div className="stat-label">今日完成率</div>
        </div>
      </div>

      <div className="stats-chart" aria-label="近 7 日提醒柱状图">
        {stats.last_days.map((d) => {
          const triggeredH = (d.triggered / max) * 100
          const completedH = (d.completed / max) * 100
          const weekday = new Date(d.date).getDay()
          return (
            <div key={d.date} className="bar-col" title={`${d.date} 触发 ${d.triggered} 完成 ${d.completed}`}>
              <div className="bar-stack">
                <div className="bar bar-triggered" style={{ height: `${triggeredH}%` }} />
                <div className="bar bar-completed" style={{ height: `${completedH}%` }} />
              </div>
              <div className="bar-label">{weekdayNames[weekday]}</div>
            </div>
          )
        })}
      </div>

      <div className="stats-legend">
        <span className="legend-dot legend-triggered" /> 触发
        <span className="legend-dot legend-completed" /> 完成
      </div>
    </section>
  )
}

export default Settings
