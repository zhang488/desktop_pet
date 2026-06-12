// 查询桌宠窗口的屏幕位置与可见状态
const pages = await (await fetch('http://127.0.0.1:9222/json/list')).json()

async function evalIn(wsUrl, expression, awaitPromise = false) {
  const ws = new WebSocket(wsUrl)
  await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej })
  const result = await new Promise((res) => {
    ws.onmessage = (ev) => res(JSON.parse(ev.data))
    ws.send(JSON.stringify({ id: 1, method: 'Runtime.evaluate', params: { expression, awaitPromise, returnByValue: true } }))
  })
  ws.close()
  return result.result?.result?.value
}

for (const p of pages.filter((x) => x.type === 'page')) {
  const label = await evalIn(p.webSocketDebuggerUrl, `window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label`)
  if (label !== 'pet') continue
  const info = await evalIn(
    p.webSocketDebuggerUrl,
    `(async () => {
      const { getCurrentWindow } = window.__TAURI_INTERNALS__ ? await import('http://localhost:1420/node_modules/.vite/deps/@tauri-apps_api_window.js').catch(() => ({})) : {}
      return JSON.stringify({
        screenX: window.screenX, screenY: window.screenY,
        w: window.outerWidth, h: window.outerHeight,
        container: !!document.querySelector('.pet-container'),
        phase: document.querySelector('.pet-container')?.className ?? null,
      })
    })()`,
    true,
  )
  console.log('pet window:', info)
}
