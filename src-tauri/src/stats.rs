use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{Duration as ChronoDuration, Local, NaiveDate};
use rusqlite::{params, Connection};
use serde::Serialize;

pub struct Db {
  conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Copy)]
pub enum EventKind {
  Trigger,
  Complete,
  Skip,
  Snooze,
  Dismiss,
}

impl EventKind {
  fn as_str(&self) -> &'static str {
    match self {
      EventKind::Trigger => "trigger",
      EventKind::Complete => "complete",
      EventKind::Skip => "skip",
      EventKind::Snooze => "snooze",
      EventKind::Dismiss => "dismiss",
    }
  }
}

#[derive(Serialize)]
pub struct TodayCount {
  pub kind: String,
  pub triggered: i64,
  pub completed: i64,
}

#[derive(Serialize)]
pub struct DailyPoint {
  pub date: String,
  pub triggered: i64,
  pub completed: i64,
}

#[derive(Serialize)]
pub struct Stats {
  pub today: Vec<TodayCount>,
  pub last_days: Vec<DailyPoint>,
  pub streak_days: i64,
  pub total_completed: i64,
}

impl Db {
  pub fn open(path: PathBuf) -> rusqlite::Result<Self> {
    if let Some(parent) = path.parent() {
      let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
      "
      CREATE TABLE IF NOT EXISTS events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        kind TEXT NOT NULL,
        event TEXT NOT NULL,
        ts INTEGER NOT NULL,
        date TEXT NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_events_date ON events(date);
      CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
      ",
    )?;
    Ok(Self {
      conn: Mutex::new(conn),
    })
  }

  pub fn record(&self, kind: &str, event: EventKind) {
    let now = Local::now();
    let ts = now.timestamp();
    let date = now.date_naive().format("%Y-%m-%d").to_string();
    let conn = match self.conn.lock() {
      Ok(c) => c,
      Err(_) => return,
    };
    let _ = conn.execute(
      "INSERT INTO events (kind, event, ts, date) VALUES (?1, ?2, ?3, ?4)",
      params![kind, event.as_str(), ts, date],
    );
  }

  pub fn stats(&self, days: i64) -> rusqlite::Result<Stats> {
    let conn = self
      .conn
      .lock()
      .map_err(|_| rusqlite::Error::InvalidQuery)?;

    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();

    // 今日各类型 trigger/complete 计数
    let mut stmt = conn.prepare(
      "SELECT kind,
              SUM(CASE WHEN event='trigger' THEN 1 ELSE 0 END),
              SUM(CASE WHEN event='complete' THEN 1 ELSE 0 END)
       FROM events
       WHERE date = ?1
       GROUP BY kind",
    )?;
    let today_rows = stmt.query_map(params![today], |row| {
      Ok(TodayCount {
        kind: row.get(0)?,
        triggered: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
        completed: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
      })
    })?;
    let today_counts: Vec<TodayCount> = today_rows.filter_map(|r| r.ok()).collect();

    // 最近 N 天的每日触发 / 完成总数
    let start_date = Local::now()
      .date_naive()
      .checked_sub_signed(ChronoDuration::days(days - 1))
      .map(|d| d.format("%Y-%m-%d").to_string())
      .unwrap_or_else(|| today.clone());

    let mut stmt2 = conn.prepare(
      "SELECT date,
              SUM(CASE WHEN event='trigger' THEN 1 ELSE 0 END),
              SUM(CASE WHEN event='complete' THEN 1 ELSE 0 END)
       FROM events
       WHERE date >= ?1
       GROUP BY date
       ORDER BY date ASC",
    )?;
    let daily_rows = stmt2.query_map(params![start_date], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, Option<i64>>(1)?.unwrap_or(0),
        row.get::<_, Option<i64>>(2)?.unwrap_or(0),
      ))
    })?;
    let mut daily_map: std::collections::HashMap<String, (i64, i64)> =
      std::collections::HashMap::new();
    for row in daily_rows.flatten() {
      daily_map.insert(row.0, (row.1, row.2));
    }
    // 补齐空白日期
    let mut last_days: Vec<DailyPoint> = Vec::new();
    let today_naive = Local::now().date_naive();
    for i in 0..days {
      if let Some(d) = today_naive.checked_sub_signed(ChronoDuration::days(days - 1 - i)) {
        let key = d.format("%Y-%m-%d").to_string();
        let (t, c) = daily_map.get(&key).copied().unwrap_or((0, 0));
        last_days.push(DailyPoint {
          date: key,
          triggered: t,
          completed: c,
        });
      }
    }

    // 连续打卡天数：从今天起向前看，只要那天有任意 complete 事件就算打卡
    let streak_days = compute_streak(&conn)?;

    // 累计完成
    let total_completed: i64 = conn
      .query_row(
        "SELECT COUNT(*) FROM events WHERE event='complete'",
        [],
        |r| r.get(0),
      )
      .unwrap_or(0);

    Ok(Stats {
      today: today_counts,
      last_days,
      streak_days,
      total_completed,
    })
  }
}

fn compute_streak(conn: &Connection) -> rusqlite::Result<i64> {
  let mut stmt = conn.prepare(
    "SELECT DISTINCT date FROM events WHERE event='complete' ORDER BY date DESC",
  )?;
  let dates: Vec<String> = stmt
    .query_map([], |row| row.get::<_, String>(0))?
    .filter_map(|r| r.ok())
    .collect();

  let today = Local::now().date_naive();
  let mut expect = today;
  let mut streak = 0i64;
  for d in dates {
    let Ok(date) = NaiveDate::parse_from_str(&d, "%Y-%m-%d") else {
      continue;
    };
    if date == expect {
      streak += 1;
      expect = match expect.checked_sub_signed(ChronoDuration::days(1)) {
        Some(d) => d,
        None => break,
      };
    } else if date < expect {
      // 早于预期 → 断流
      break;
    }
    // date > expect 不该发生（因 ORDER BY DESC），忽略
  }
  Ok(streak)
}
