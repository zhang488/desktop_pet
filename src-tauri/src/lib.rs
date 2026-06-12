mod stats;

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::sync::Arc;
use std::sync::Mutex;

use chrono::{Duration as ChronoDuration, Local, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use tauri::{
  menu::{Menu, MenuItem, Submenu},
  tray::TrayIconBuilder,
  AppHandle, Emitter, Manager, PhysicalPosition, State, WindowEvent,
};
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};

use stats::{Db, EventKind};

const PET_WINDOW_LABEL: &str = "pet";
const SETTINGS_WINDOW_LABEL: &str = "settings";
const WINDOW_POS_FILE: &str = "window.json";
const SETTINGS_FILE: &str = "settings.json";

#[derive(Clone, Copy, Debug)]
enum Schedule {
  Interval(u64),
  DailyAt(u32, u32),
}

#[derive(Clone)]
struct ReminderTaskConfig {
  id: &'static str,
  name: &'static str,
  schedule: Schedule,
  messages: &'static [&'static str],
}

fn reminder_tasks() -> &'static [ReminderTaskConfig] {
  &[
    ReminderTaskConfig {
      id: "drink_water",
      name: "喝水",
      schedule: Schedule::Interval(60),
      messages: &[
        "该喝水啦！喝口水休息一下吧 💧",
        "水分不足啦～来一口 🫗",
        "小提醒：喝水时间到了 💦",
        "记得补水哦，一口就好 🥤",
      ],
    },
    ReminderTaskConfig {
      id: "eye_strain",
      name: "护眼休息",
      schedule: Schedule::Interval(90),
      messages: &[
        "眼睛累不累？看看远处吧 👀",
        "护眼时间到，眺望远方 20 秒 🌄",
        "屏幕看太久啦，眨眨眼放松一下 ✨",
      ],
    },
    ReminderTaskConfig {
      id: "posture",
      name: "久坐起身",
      schedule: Schedule::Interval(120),
      messages: &[
        "起来活动一下，久坐对腰不好 🪑",
        "站起来伸个懒腰吧 🧘",
        "动一动，让身体醒过来 💪",
      ],
    },
    ReminderTaskConfig {
      id: "lunch",
      name: "吃午饭",
      schedule: Schedule::DailyAt(12, 0),
      messages: &[
        "中午啦，记得吃饭哦 🍱",
        "干饭时间，别错过啦 🍚",
        "好好吃饭，下午才有力气 🥢",
      ],
    },
    ReminderTaskConfig {
      id: "sleep",
      name: "睡觉",
      schedule: Schedule::DailyAt(22, 30),
      messages: &[
        "夜深啦，准备睡觉吧 🌙",
        "今天辛苦啦，早点休息 😴",
        "晚安，明天见 ⭐",
      ],
    },
  ]
}

// 数字越小优先级越高（弹出顺序越靠前）
fn task_priority(id: &str) -> u8 {
  match id {
    "sleep" => 1,
    "lunch" => 2,
    "eye_strain" => 3,
    "drink_water" => 4,
    "posture" => 5,
    _ => 99,
  }
}

#[derive(Clone, Serialize, Deserialize)]
struct TaskOverride {
  enabled: bool,
  interval_secs: Option<u64>,
  daily_hour: Option<u32>,
  daily_minute: Option<u32>,
}

impl Default for TaskOverride {
  fn default() -> Self {
    Self {
      enabled: true,
      interval_secs: None,
      daily_hour: None,
      daily_minute: None,
    }
  }
}

#[derive(Clone, Serialize, Deserialize)]
struct DndConfig {
  enabled: bool,
  start_hour: u32,
  start_minute: u32,
  end_hour: u32,
  end_minute: u32,
}

impl Default for DndConfig {
  fn default() -> Self {
    Self {
      enabled: false,
      start_hour: 23,
      start_minute: 0,
      end_hour: 7,
      end_minute: 0,
    }
  }
}

#[derive(Serialize, Deserialize, Default)]
struct PersistedSettings {
  tasks: HashMap<String, TaskOverride>,
  #[serde(default)]
  dnd: DndConfig,
}

struct AppState {
  paused: bool,
  active_reminder: Option<String>,
  /// 当前未响应提醒的完整内容（供窗口刷新后重新拉取，防事件丢失）
  active_payload: Option<ReminderPayload>,
  /// active 变更代数，每次设置/清除 +1，看门狗用它判断提醒是否还停留在原地
  active_gen: u64,
  pending_queue: VecDeque<String>,
  tasks: HashMap<String, TaskOverride>,
  dnd: DndConfig,
}

impl AppState {
  fn task_enabled(&self, id: &str) -> bool {
    self.tasks.get(id).map(|t| t.enabled).unwrap_or(true)
  }

  fn task_interval(&self, id: &str, default: u64) -> u64 {
    self
      .tasks
      .get(id)
      .and_then(|t| t.interval_secs)
      .unwrap_or(default)
  }

  fn task_daily(&self, id: &str, default: (u32, u32)) -> (u32, u32) {
    let t = self.tasks.get(id);
    let h = t.and_then(|t| t.daily_hour).unwrap_or(default.0);
    let m = t.and_then(|t| t.daily_minute).unwrap_or(default.1);
    (h, m)
  }

  fn snapshot(&self) -> PersistedSettings {
    PersistedSettings {
      tasks: self.tasks.clone(),
      dnd: self.dnd.clone(),
    }
  }
}

impl Default for AppState {
  fn default() -> Self {
    let mut tasks: HashMap<String, TaskOverride> = HashMap::new();
    for t in reminder_tasks() {
      tasks.insert(t.id.to_string(), TaskOverride::default());
    }
    Self {
      paused: false,
      active_reminder: None,
      active_payload: None,
      active_gen: 0,
      pending_queue: VecDeque::new(),
      tasks,
      dnd: DndConfig::default(),
    }
  }
}

#[derive(Clone, Serialize)]
struct ReminderPayload {
  kind: String,
  name: String,
  message: String,
  timestamp: i64,
}

#[derive(Serialize, Deserialize, Default)]
struct WindowPos {
  x: i32,
  y: i32,
}

#[derive(Serialize)]
struct TaskInfo {
  id: String,
  name: String,
  enabled: bool,
  kind: String,
  interval_secs: Option<u64>,
  daily_hour: Option<u32>,
  daily_minute: Option<u32>,
}

#[derive(Serialize)]
struct GlobalSettings {
  paused: bool,
  dnd: DndConfig,
  autostart: bool,
}

fn pick_message(messages: &[&str]) -> String {
  if messages.is_empty() {
    return "提醒来啦～".to_string();
  }
  let idx = (chrono::Utc::now().timestamp_millis() as usize) % messages.len();
  messages[idx].to_string()
}

fn config_path(app: &AppHandle, file: &str) -> Option<std::path::PathBuf> {
  let dir = app.path().app_config_dir().ok()?;
  Some(dir.join(file))
}

fn load_settings(app: &AppHandle) -> PersistedSettings {
  let Some(path) = config_path(app, SETTINGS_FILE) else {
    return PersistedSettings::default();
  };
  fs::read_to_string(path)
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default()
}

fn save_settings(app: &AppHandle, settings: &PersistedSettings) {
  let Some(path) = config_path(app, SETTINGS_FILE) else { return };
  if let Some(parent) = path.parent() {
    let _ = fs::create_dir_all(parent);
  }
  if let Ok(json) = serde_json::to_string_pretty(settings) {
    let _ = fs::write(path, json);
  }
}

fn load_window_pos(app: &AppHandle) -> Option<WindowPos> {
  let path = config_path(app, WINDOW_POS_FILE)?;
  let data = fs::read_to_string(path).ok()?;
  serde_json::from_str(&data).ok()
}

fn save_window_pos(app: &AppHandle, pos: &WindowPos) {
  let Some(path) = config_path(app, WINDOW_POS_FILE) else { return };
  if let Some(parent) = path.parent() {
    let _ = fs::create_dir_all(parent);
  }
  if let Ok(json) = serde_json::to_string(pos) {
    let _ = fs::write(path, json);
  }
}

fn is_dnd_now(dnd: &DndConfig) -> bool {
  if !dnd.enabled {
    return false;
  }
  let now = Local::now();
  let now_mins = now.hour() * 60 + now.minute();
  let start = dnd.start_hour * 60 + dnd.start_minute;
  let end = dnd.end_hour * 60 + dnd.end_minute;
  if start == end {
    return false;
  }
  if start < end {
    now_mins >= start && now_mins < end
  } else {
    now_mins >= start || now_mins < end
  }
}

fn read_active(app: &AppHandle) -> bool {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(s) => s.active_reminder.is_some(),
    Err(_) => false,
  }
}

fn read_dnd(app: &AppHandle) -> DndConfig {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(s) => s.dnd.clone(),
    Err(_) => DndConfig::default(),
  }
}

/// 设置 active；成功时返回本次的代数（看门狗用）
fn try_set_active(app: &AppHandle, id: &str, force: bool, payload: &ReminderPayload) -> Option<u64> {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(mut s) => {
      if s.active_reminder.is_some() && !force {
        None
      } else {
        s.active_reminder = Some(id.to_string());
        s.active_payload = Some(payload.clone());
        s.active_gen += 1;
        Some(s.active_gen)
      }
    }
    Err(_) => None,
  }
}

fn read_active_gen(app: &AppHandle) -> u64 {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(s) => s.active_gen,
    Err(_) => 0,
  }
}

fn read_paused(app: &AppHandle) -> bool {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(s) => s.paused,
    Err(_) => false,
  }
}

fn enqueue_pending(app: &AppHandle, id: &str) {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  if let Ok(mut s) = guard {
    if !s.pending_queue.iter().any(|x| x == id) {
      s.pending_queue.push_back(id.to_string());
      // 按优先级稳定排序
      let mut v: Vec<String> = s.pending_queue.drain(..).collect();
      v.sort_by_key(|x| task_priority(x));
      s.pending_queue.extend(v);
    }
  }
}

fn pop_next_pending(app: &AppHandle) -> Option<String> {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(mut s) => s.pending_queue.pop_front(),
    Err(_) => None,
  }
}

fn flush_queue(app: &AppHandle) {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  if let Ok(mut s) = guard {
    s.pending_queue.clear();
  }
}

fn toggle_paused_tray(app: &AppHandle) {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  if let Ok(mut s) = guard {
    s.paused = !s.paused;
    if s.paused {
      s.pending_queue.clear();
    }
    log::info!("scheduler paused = {}", s.paused);
  }
}

fn take_active(app: &AppHandle) -> Option<String> {
  let state = app.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(mut s) => {
      s.active_payload = None;
      s.active_gen += 1;
      s.active_reminder.take()
    }
    Err(_) => None,
  }
}

fn record_event(app: &AppHandle, kind: &str, event: EventKind) {
  if let Some(db) = app.try_state::<Arc<Db>>() {
    db.record(kind, event);
  }
}

/// 响应（complete/skip/snooze/hide）后：清 active，若队列有下一项则延迟弹出
fn after_response(app: &AppHandle, event: EventKind) {
  let active = take_active(app);
  if let Some(id) = active {
    record_event(app, &id, event);
  }
  if let Some(next) = pop_next_pending(app) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
      tokio::time::sleep(std::time::Duration::from_millis(900)).await;
      trigger_reminder_by_id(&handle, &next, true);
    });
  }
}

/// 提醒弹出后无响应的最长时间，超时按"忽略"自动收起，防止 active 卡死
const REMINDER_WATCHDOG_SECS: u64 = 120;

fn try_trigger_auto(app: &AppHandle, id: &str) {
  // 暂停中：直接丢弃（snooze 回调等异步路径也走这里）
  if read_paused(app) {
    log::info!("dropping {} — scheduler paused", id);
    return;
  }
  // 勿扰时段：直接跳过，不入队（避免出 DnD 后涌出）
  if is_dnd_now(&read_dnd(app)) {
    log::info!("dropping {} — in DnD window", id);
    return;
  }
  if read_active(app) {
    log::info!("queueing {} — another reminder is active", id);
    enqueue_pending(app, id);
    return;
  }
  trigger_reminder_by_id(app, id, false);
}

fn trigger_reminder_by_id(app: &AppHandle, id: &str, force: bool) {
  let Some(task) = reminder_tasks().iter().find(|t| t.id == id) else {
    return;
  };
  let payload = ReminderPayload {
    kind: task.id.to_string(),
    name: task.name.to_string(),
    message: pick_message(task.messages),
    timestamp: chrono::Utc::now().timestamp(),
  };
  let Some(gen) = try_set_active(app, id, force, &payload) else {
    return;
  };
  record_event(app, id, EventKind::Trigger);
  if let Some(w) = app.get_webview_window(PET_WINDOW_LABEL) {
    let _ = w.show();
    let _ = w.set_focus();
  }
  let _ = app.emit("reminder:trigger", payload);

  // 看门狗：超时仍无响应（前端事件丢失/用户不在），自动按忽略处理并放行队列
  let handle = app.clone();
  tauri::async_runtime::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(REMINDER_WATCHDOG_SECS)).await;
    if read_active_gen(&handle) == gen {
      log::warn!("reminder watchdog: auto-dismissing stale reminder");
      if let Some(w) = handle.get_webview_window(PET_WINDOW_LABEL) {
        let _ = w.hide();
      }
      after_response(&handle, EventKind::Dismiss);
    }
  });
}

#[tauri::command]
fn show_pet(app: AppHandle) -> Result<(), String> {
  if let Some(w) = app.get_webview_window(PET_WINDOW_LABEL) {
    w.show().map_err(|e| e.to_string())?;
    let _ = w.set_focus();
  }
  Ok(())
}

#[tauri::command]
fn hide_pet(app: AppHandle) -> Result<(), String> {
  if let Some(w) = app.get_webview_window(PET_WINDOW_LABEL) {
    w.hide().map_err(|e| e.to_string())?;
  }
  after_response(&app, EventKind::Dismiss);
  Ok(())
}

#[tauri::command]
fn complete_reminder(app: AppHandle) -> Result<(), String> {
  if let Some(w) = app.get_webview_window(PET_WINDOW_LABEL) {
    w.hide().map_err(|e| e.to_string())?;
  }
  after_response(&app, EventKind::Complete);
  Ok(())
}

#[tauri::command]
fn skip_reminder(app: AppHandle) -> Result<(), String> {
  if let Some(w) = app.get_webview_window(PET_WINDOW_LABEL) {
    w.hide().map_err(|e| e.to_string())?;
  }
  after_response(&app, EventKind::Skip);
  Ok(())
}

/// 桌宠窗口加载/刷新时拉取当前未响应的提醒（防 emit 事件丢失）
#[tauri::command]
fn get_active_reminder(
  state: State<'_, Mutex<AppState>>,
) -> Result<Option<ReminderPayload>, String> {
  let s = state.lock().map_err(|e| e.to_string())?;
  Ok(s.active_payload.clone())
}

#[tauri::command]
fn get_stats(app: AppHandle, days: Option<i64>) -> Result<stats::Stats, String> {
  let d = days.unwrap_or(7).clamp(1, 90);
  let db = app
    .try_state::<Arc<Db>>()
    .ok_or_else(|| "db not initialized".to_string())?;
  db.stats(d).map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_pause(app: AppHandle, state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
  let paused_now = {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.paused = !s.paused;
    if s.paused {
      s.pending_queue.clear();
    }
    s.paused
  };
  let _ = app;
  Ok(paused_now)
}

#[tauri::command]
fn trigger_now(app: AppHandle, kind: Option<String>) -> Result<(), String> {
  let id = kind.unwrap_or_else(|| "drink_water".to_string());
  // 手动触发：强制覆盖当前 active，不入队
  trigger_reminder_by_id(&app, &id, true);
  Ok(())
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
  if let Some(w) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
    w.show().map_err(|e| e.to_string())?;
    let _ = w.set_focus();
  }
  Ok(())
}

#[tauri::command]
fn list_tasks(state: State<'_, Mutex<AppState>>) -> Result<Vec<TaskInfo>, String> {
  let s = state.lock().map_err(|e| e.to_string())?;
  let mut result = Vec::new();
  for t in reminder_tasks() {
    let info = match t.schedule {
      Schedule::Interval(default) => TaskInfo {
        id: t.id.to_string(),
        name: t.name.to_string(),
        enabled: s.task_enabled(t.id),
        kind: "interval".to_string(),
        interval_secs: Some(s.task_interval(t.id, default)),
        daily_hour: None,
        daily_minute: None,
      },
      Schedule::DailyAt(h, m) => {
        let (hh, mm) = s.task_daily(t.id, (h, m));
        TaskInfo {
          id: t.id.to_string(),
          name: t.name.to_string(),
          enabled: s.task_enabled(t.id),
          kind: "daily".to_string(),
          interval_secs: None,
          daily_hour: Some(hh),
          daily_minute: Some(mm),
        }
      }
    };
    result.push(info);
  }
  Ok(result)
}

#[tauri::command]
fn set_task_enabled(
  app: AppHandle,
  state: State<'_, Mutex<AppState>>,
  id: String,
  enabled: bool,
) -> Result<(), String> {
  let snapshot = {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let entry = s.tasks.entry(id).or_default();
    entry.enabled = enabled;
    s.snapshot()
  };
  save_settings(&app, &snapshot);
  Ok(())
}

#[tauri::command]
fn set_task_interval(
  app: AppHandle,
  state: State<'_, Mutex<AppState>>,
  id: String,
  interval_secs: u64,
) -> Result<(), String> {
  let snapshot = {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let entry = s.tasks.entry(id).or_default();
    entry.interval_secs = Some(interval_secs.max(5));
    s.snapshot()
  };
  save_settings(&app, &snapshot);
  Ok(())
}

#[tauri::command]
fn set_task_daily(
  app: AppHandle,
  state: State<'_, Mutex<AppState>>,
  id: String,
  hour: u32,
  minute: u32,
) -> Result<(), String> {
  let snapshot = {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let entry = s.tasks.entry(id).or_default();
    entry.daily_hour = Some(hour.min(23));
    entry.daily_minute = Some(minute.min(59));
    s.snapshot()
  };
  save_settings(&app, &snapshot);
  Ok(())
}

#[tauri::command]
fn get_global_settings(
  app: AppHandle,
  state: State<'_, Mutex<AppState>>,
) -> Result<GlobalSettings, String> {
  let (paused, dnd) = {
    let s = state.lock().map_err(|e| e.to_string())?;
    (s.paused, s.dnd.clone())
  };
  let autostart = app
    .autolaunch()
    .is_enabled()
    .map_err(|e| e.to_string())
    .unwrap_or(false);
  Ok(GlobalSettings {
    paused,
    dnd,
    autostart,
  })
}

#[tauri::command]
fn set_paused(
  state: State<'_, Mutex<AppState>>,
  paused: bool,
) -> Result<(), String> {
  let mut s = state.lock().map_err(|e| e.to_string())?;
  s.paused = paused;
  if paused {
    s.pending_queue.clear();
  }
  Ok(())
}

#[tauri::command]
fn set_dnd(
  app: AppHandle,
  state: State<'_, Mutex<AppState>>,
  enabled: bool,
  start_hour: u32,
  start_minute: u32,
  end_hour: u32,
  end_minute: u32,
) -> Result<(), String> {
  let snapshot = {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.dnd = DndConfig {
      enabled,
      start_hour: start_hour.min(23),
      start_minute: start_minute.min(59),
      end_hour: end_hour.min(23),
      end_minute: end_minute.min(59),
    };
    s.snapshot()
  };
  save_settings(&app, &snapshot);
  Ok(())
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
  let manager = app.autolaunch();
  if enabled {
    manager.enable().map_err(|e| e.to_string())
  } else {
    manager.disable().map_err(|e| e.to_string())
  }
}

fn seconds_until_daily(hour: u32, minute: u32) -> u64 {
  let now = Local::now();
  let today = now.date_naive();
  let target_naive = today
    .and_hms_opt(hour, minute, 0)
    .or_else(|| today.and_hms_opt(0, 0, 0));
  let Some(target_naive) = target_naive else {
    return 60;
  };
  let target = match Local.from_local_datetime(&target_naive).single() {
    Some(t) => t,
    None => return 60,
  };
  let target = if target <= now {
    target + ChronoDuration::days(1)
  } else {
    target
  };
  (target - now).num_seconds().max(1) as u64
}

fn read_interval(handle: &AppHandle, id: &str, default: u64) -> u64 {
  let state = handle.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(g) => g.task_interval(id, default),
    Err(_) => default,
  }
}

fn read_daily(handle: &AppHandle, id: &str, default: (u32, u32)) -> (u32, u32) {
  let state = handle.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(g) => g.task_daily(id, default),
    Err(_) => default,
  }
}

fn read_runtime(handle: &AppHandle, id: &str) -> (bool, bool) {
  let state = handle.state::<Mutex<AppState>>();
  let guard = state.lock();
  match guard {
    Ok(g) => (g.paused, g.task_enabled(id)),
    Err(_) => (false, true),
  }
}

fn start_interval_task(handle: AppHandle, task: ReminderTaskConfig, default_secs: u64) {
  tauri::async_runtime::spawn(async move {
    let initial = read_interval(&handle, task.id, default_secs);
    tokio::time::sleep(std::time::Duration::from_secs(initial)).await;
    loop {
      let (paused, enabled) = read_runtime(&handle, task.id);
      if !paused && enabled {
        try_trigger_auto(&handle, task.id);
      }
      let next_secs = read_interval(&handle, task.id, default_secs).max(5);
      tokio::time::sleep(std::time::Duration::from_secs(next_secs)).await;
    }
  });
}

fn start_daily_task(handle: AppHandle, task: ReminderTaskConfig, default_hm: (u32, u32)) {
  tauri::async_runtime::spawn(async move {
    loop {
      let (h, m) = read_daily(&handle, task.id, default_hm);
      let wait = seconds_until_daily(h, m);
      tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
      let (paused, enabled) = read_runtime(&handle, task.id);
      if !paused && enabled {
        try_trigger_auto(&handle, task.id);
      }
      tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
  });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
      if let Some(w) = app.get_webview_window(PET_WINDOW_LABEL) {
        let _ = w.show();
        let _ = w.set_focus();
      }
    }))
    .plugin(tauri_plugin_autostart::init(
      MacosLauncher::LaunchAgent,
      None,
    ))
    .manage(Mutex::new(AppState::default()))
    .invoke_handler(tauri::generate_handler![
      show_pet,
      hide_pet,
      complete_reminder,
      skip_reminder,
      toggle_pause,
      trigger_now,
      open_settings,
      list_tasks,
      set_task_enabled,
      set_task_interval,
      set_task_daily,
      get_global_settings,
      set_paused,
      set_dnd,
      set_autostart,
      get_stats,
      get_active_reminder,
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // 初始化 SQLite
      let db_path = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("desktop-pet.db"));
      if let Some(path) = db_path {
        match Db::open(path) {
          Ok(db) => {
            app.manage(Arc::new(db));
          }
          Err(e) => {
            log::error!("failed to init sqlite: {}", e);
          }
        }
      }

      // 加载持久化设置
      let persisted = load_settings(app.handle());
      {
        let state = app.state::<Mutex<AppState>>();
        let guard = state.lock();
        if let Ok(mut s) = guard {
          for (id, ov) in persisted.tasks {
            s.tasks.insert(id, ov);
          }
          s.dnd = persisted.dnd;
        }
      }

      // 启动时恢复上次的窗口位置。
      // 注意：Moved 事件保存的是物理像素，必须用 PhysicalPosition 恢复；
      // 之前误用 LogicalPosition，在高 DPI 缩放下每次重启位置会乘以缩放系数，最终漂出屏幕外。
      let handle = app.handle().clone();
      if let (Some(pos), Some(w)) = (
        load_window_pos(&handle),
        handle.get_webview_window(PET_WINDOW_LABEL),
      ) {
        let (mut x, mut y) = (pos.x, pos.y);
        // 历史坏数据 / 显示器变更兜底：把整个窗口拉回主屏可见范围
        if let Ok(Some(monitor)) = w.primary_monitor() {
          let mp = monitor.position();
          let ms = monitor.size();
          let (ww, wh) = w
            .outer_size()
            .map(|s| (s.width as i32, s.height as i32))
            .unwrap_or((540, 720));
          x = x.clamp(mp.x, (mp.x + ms.width as i32 - ww).max(mp.x));
          y = y.clamp(mp.y, (mp.y + ms.height as i32 - wh).max(mp.y));
        }
        let _ = w.set_position(PhysicalPosition::new(x, y));
      }

      // 监听桌宠窗口事件：移动时保存位置；Alt+F4 等关闭请求改为隐藏（窗口被销毁就再也弹不出来了）
      let handle_for_move = app.handle().clone();
      if let Some(w) = handle_for_move.get_webview_window(PET_WINDOW_LABEL) {
        w.on_window_event(move |event| match event {
          WindowEvent::Moved(PhysicalPosition { x, y }) => {
            save_window_pos(&handle_for_move, &WindowPos { x: *x, y: *y });
          }
          WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(w) = handle_for_move.get_webview_window(PET_WINDOW_LABEL) {
              let _ = w.hide();
            }
            after_response(&handle_for_move, EventKind::Dismiss);
          }
          _ => {}
        });
      }

      // 构建托盘菜单
      let show_item = MenuItem::with_id(app, "show", "显示桌宠", true, None::<&str>)?;
      let pause_item = MenuItem::with_id(app, "pause", "暂停 / 恢复全部", true, None::<&str>)?;
      let settings_item = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
      let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

      let mut trigger_items: Vec<MenuItem<tauri::Wry>> = Vec::new();
      for t in reminder_tasks() {
        let item = MenuItem::with_id(
          app,
          format!("trigger:{}", t.id),
          format!("立即提醒：{}", t.name),
          true,
          None::<&str>,
        )?;
        trigger_items.push(item);
      }
      let trigger_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = trigger_items
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect();
      let trigger_submenu = Submenu::with_items(app, "立即触发", true, &trigger_refs)?;

      let menu = Menu::with_items(
        app,
        &[
          &show_item,
          &trigger_submenu,
          &pause_item,
          &settings_item,
          &quit_item,
        ],
      )?;

      let icon = app
        .default_window_icon()
        .ok_or("default window icon missing")?
        .clone();

      let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .tooltip("Desktop Pet")
        .on_menu_event(|app, event| {
          let id = event.id.as_ref();
          match id {
            "show" => {
              if let Some(w) = app.get_webview_window(PET_WINDOW_LABEL) {
                let _ = w.show();
                let _ = w.set_focus();
              }
            }
            "pause" => {
              toggle_paused_tray(app);
            }
            "settings" => {
              if let Some(w) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
                let _ = w.show();
                let _ = w.set_focus();
              }
            }
            "quit" => {
              app.exit(0);
            }
            other if other.starts_with("trigger:") => {
              let kind = &other[8..];
              trigger_reminder_by_id(app, kind, true);
            }
            _ => {}
          }
        })
        .build(app)?;

      // 为每个提醒任务启动独立的 tokio 调度
      for task in reminder_tasks() {
        let handle = app.handle().clone();
        let task = task.clone();
        match task.schedule {
          Schedule::Interval(secs) => start_interval_task(handle, task, secs),
          Schedule::DailyAt(h, m) => start_daily_task(handle, task, (h, m)),
        }
      }

      // 设置窗口关闭时改为隐藏，下次打开复用
      let handle_settings = app.handle().clone();
      if let Some(sw) = handle_settings.get_webview_window(SETTINGS_WINDOW_LABEL) {
        let sw_clone = sw.clone();
        sw.on_window_event(move |event| {
          if let WindowEvent::CloseRequested { api, .. } = event {
            let _ = sw_clone.hide();
            api.prevent_close();
          }
        });
      }

      let _ = flush_queue; // 避免 dead_code 警告（保留以备后用）

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
