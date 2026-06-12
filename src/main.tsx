import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { getCurrentWindow } from '@tauri-apps/api/window'

const label = getCurrentWindow().label

const root = createRoot(document.getElementById('root')!)

if (label === 'settings') {
  // 设置窗口：独立 bundle，不污染桌宠窗口的透明背景
  Promise.all([
    import('./pages/Settings'),
    import('./pages/Settings.css'),
  ]).then(([{ default: Settings }]) => {
    root.render(
      <StrictMode>
        <Settings />
      </StrictMode>,
    )
  })
} else {
  Promise.all([
    import('./App'),
    import('./index.css'),
  ]).then(([{ default: App }]) => {
    root.render(
      <StrictMode>
        <App />
      </StrictMode>,
    )
  })
}
