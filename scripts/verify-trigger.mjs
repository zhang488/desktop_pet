// 端到端验证：通过 CDP 在设置窗口里调用 trigger_now（等价于点击「立即测试」），
// 然后检查桌宠窗口是否渲染出了提醒 UI。
// 用法: node --experimental-websocket scripts/verify-trigger.mjs

const DEBUG_HOST = 'http://127.0.0.1:9222'

async function cdp(wsUrl) {
  const ws = new WebSocket(wsUrl)
  await new Promise((res, rej) => {
    ws.onopen = res
    ws.onerror = rej
  })
  let id = 0
  const pending = new Map()
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data)
    if (msg.id && pending.has(msg.id)) {
      pending.get(msg.id)(msg)
      pending.delete(msg.id)
    }
  }
  return {
    send(method, params = {}) {
      const msgId = ++id
      ws.send(JSON.stringify({ id: msgId, method, params }))
      return new Promise((res) => pending.set(msgId, res))
    },
    close() {
      ws.close()
    },
  }
}

async function evalIn(client, expression, awaitPromise = false) {
  const r = await client.send('Runtime.evaluate', {
    expression,
    awaitPromise,
    returnByValue: true,
  })
  if (r.result?.exceptionDetails) {
    return { error: r.result.exceptionDetails.exception?.description ?? 'eval error' }
  }
  return { value: r.result?.result?.value }
}

const pages = await (await fetch(`${DEBUG_HOST}/json/list`)).json()
const targets = pages.filter((p) => p.type === 'page')
console.log(`found ${targets.length} pages`)

// 1. 识别每个页面属于哪个窗口
const labeled = []
for (const t of targets) {
  const c = await cdp(t.webSocketDebuggerUrl)
  const r = await evalIn(
    c,
    `window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label ?? 'unknown'`,
  )
  labeled.push({ label: r.value, client: c, error: r.error })
  console.log(`page ${t.id.slice(0, 8)} → label = ${JSON.stringify(r)}`)
}

const settings = labeled.find((p) => p.label === 'settings')
const pet = labeled.find((p) => p.label === 'pet')
if (!settings || !pet) {
  console.error('FAIL: could not identify settings/pet windows')
  process.exit(1)
}

// 2. 桌宠窗口触发前的状态
const before = await evalIn(
  pet.client,
  `({ container: !!document.querySelector('.pet-container'), text: document.body.innerText.slice(0, 100) })`,
)
console.log('pet BEFORE trigger:', JSON.stringify(before))

// 3. 在设置窗口里调用 trigger_now（与「立即测试」按钮同一路径）
const invoke = await evalIn(
  settings.client,
  `window.__TAURI_INTERNALS__.invoke('trigger_now', { kind: 'drink_water' }).then(() => 'ok')`,
  true,
)
console.log('invoke trigger_now:', JSON.stringify(invoke))

// 4. 等动画/事件落地后检查桌宠窗口 DOM
await new Promise((r) => setTimeout(r, 1500))
const after = await evalIn(
  pet.client,
  `({
    container: !!document.querySelector('.pet-container'),
    phase: document.querySelector('.pet-container')?.className ?? null,
    bubble: document.querySelector('.pet-bubble-text')?.textContent ?? null,
    visible: document.visibilityState,
  })`,
)
console.log('pet AFTER trigger:', JSON.stringify(after))

// 5. 再验证 get_active_reminder 兜底命令
const active = await evalIn(
  pet.client,
  `window.__TAURI_INTERNALS__.invoke('get_active_reminder').then((p) => JSON.stringify(p))`,
  true,
)
console.log('get_active_reminder:', JSON.stringify(active))

const ok = after.value?.container && after.value?.bubble
console.log(ok ? 'RESULT: PASS — 桌宠已弹出并显示提醒' : 'RESULT: FAIL — 桌宠未渲染提醒')

for (const p of labeled) p.client.close()
process.exit(ok ? 0 : 1)
