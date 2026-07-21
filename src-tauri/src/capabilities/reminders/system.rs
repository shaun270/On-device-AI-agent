//! macOS Reminders via EventKit (native API — much faster than AppleScript).

use chrono::{DateTime, Datelike, Local, NaiveDate, Timelike, TimeZone};

/// Flexible query for listing reminders — filled by the LLM from natural language.
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    pub limit: u32,
    pub window_start: Option<u32>,
    pub window_end: Option<u32>,
    pub include_undated: bool,
    pub include_overdue: bool,
    pub include_completed: bool,
    pub search: Option<String>,
    pub list_name: Option<String>,
}

/// Build a [`ListQuery`] from tool JSON.
pub fn list_query_from_json(input: &serde_json::Value) -> Result<ListQuery, String> {
    let limit = input
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32)
        .unwrap_or(25)
        .clamp(1, 200);

    let search = input
        .get("search")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let list_name = input
        .get("list_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let include_completed = input
        .get("include_completed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let today = today_ymd()?;
    let today_ord = ymd_ord(today.0, today.1, today.2);

    let start_iso = input.get("start").and_then(|v| v.as_str()).map(str::trim);
    let end_iso = input.get("end").and_then(|v| v.as_str()).map(str::trim);
    let days_ahead = input.get("days_ahead").and_then(|v| v.as_u64()).map(|n| n as u32);
    let range = input
        .get("range")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    let (window_start, window_end, windowed) = if start_iso.is_some() || end_iso.is_some() {
        let start = match start_iso {
            Some(s) if !s.is_empty() => Some(parse_ymd(s)?),
            _ => None,
        };
        let end = match end_iso {
            Some(s) if !s.is_empty() => Some(parse_ymd(s)?),
            _ => None,
        };
        let start_ord = start.map(|(y, m, d)| ymd_ord(y, m, d));
        let end_ord = match end {
            Some((y, m, d)) => Some(ymd_ord_plus_days(y, m, d, 1)?),
            None => None,
        };
        (start_ord, end_ord, true)
    } else if let Some(n) = days_ahead {
        let end = ymd_plus_days(today.0, today.1, today.2, n as i32)?;
        (
            Some(today_ord),
            Some(ymd_ord(end.0, end.1, end.2)),
            true,
        )
    } else {
        match range.as_deref() {
            Some("today") | Some("day") => {
                let tomorrow = ymd_plus_days(today.0, today.1, today.2, 1)?;
                (
                    Some(today_ord),
                    Some(ymd_ord(tomorrow.0, tomorrow.1, tomorrow.2)),
                    true,
                )
            }
            Some("week") | Some("weekly") => {
                let end = ymd_plus_days(today.0, today.1, today.2, 7)?;
                (
                    Some(today_ord),
                    Some(ymd_ord(end.0, end.1, end.2)),
                    true,
                )
            }
            Some("all") | Some("everything") | None => (None, None, false),
            Some(other) => {
                return Err(format!(
                    "unknown range \"{other}\"; use all, today, week, days_ahead, or start/end"
                ));
            }
        }
    };

    let include_undated = input
        .get("include_undated")
        .and_then(|v| v.as_bool())
        .unwrap_or(!windowed);

    let include_overdue = input
        .get("include_overdue")
        .and_then(|v| v.as_bool())
        .unwrap_or(windowed);

    Ok(ListQuery {
        limit,
        window_start,
        window_end,
        include_undated,
        include_overdue,
        include_completed,
        search,
        list_name,
    })
}

#[cfg(target_os = "macos")]
pub fn create_reminder(
    title: &str,
    due_iso: Option<&str>,
    notes: Option<&str>,
    list_name: Option<&str>,
) -> Result<String, String> {
    use eventkit::ReminderDraft;

    let title = title.trim();
    if title.is_empty() {
        return Err("title must not be empty".into());
    }

    let eventkit_result = with_reminders(|mgr| {
        mgr.ensure_authorized().map_err(map_ek_err)?;

        let due_date = match due_iso.map(str::trim).filter(|s| !s.is_empty()) {
            Some(raw) => Some(parse_due_datetime(raw)?),
            None => None,
        };

        let draft = ReminderDraft {
            title,
            notes: notes.map(str::trim).filter(|s| !s.is_empty()),
            calendar_title: list_name.map(str::trim).filter(|s| !s.is_empty()),
            due_date,
            ..Default::default()
        };

        let item = mgr.create_reminder(&draft).map_err(map_ek_err)?;

        let mut msg = format!("Created reminder \"{}\"", item.title);
        if let Some(d) = item.due_date {
            msg.push_str(&format!(" due {}", d.format("%Y-%m-%d %H:%M")));
        }
        if let Some(list) = item.calendar_title {
            msg.push_str(&format!(" in list \"{list}\""));
        }
        Ok(msg)
    });

    match eventkit_result {
        Ok(msg) => Ok(msg),
        Err(e) if is_auth_failure(&e) => {
            // `tauri dev` often never appears under Privacy → Reminders.
            // AppleScript uses Automation instead and still works.
            create_reminder_applescript(title, due_iso, notes)
        }
        Err(e) => Err(e),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn create_reminder(
    _title: &str,
    _due_iso: Option<&str>,
    _notes: Option<&str>,
    _list_name: Option<&str>,
) -> Result<String, String> {
    Err("reminders are only supported on macOS".into())
}

#[cfg(target_os = "macos")]
pub fn list_reminders(query: &ListQuery) -> Result<String, String> {
    let eventkit_result = with_reminders(|mgr| {
        mgr.ensure_authorized().map_err(map_ek_err)?;

        let cal_titles: Option<Vec<String>> =
            query.list_name.as_ref().map(|s| vec![s.clone()]);
        let cal_refs: Option<Vec<&str>> = cal_titles
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let cal_slice = cal_refs.as_ref().map(|v| v.as_slice());

        let mut items = if query.include_completed {
            mgr.fetch_reminders(cal_slice).map_err(map_ek_err)?
        } else if query.window_start.is_some() || query.window_end.is_some() {
            let start = query
                .window_start
                .map(ord_to_local_midnight)
                .transpose()?;
            let end = query.window_end.map(ord_to_local_midnight).transpose()?;
            let mut dated = mgr
                .fetch_incomplete_reminders_in_due_range(start, end, cal_slice)
                .map_err(map_ek_err)?;
            if query.include_undated {
                let undated: Vec<_> = mgr
                    .fetch_incomplete_reminders()
                    .map_err(map_ek_err)?
                    .into_iter()
                    .filter(|r| r.due_date.is_none())
                    .filter(|r| calendar_matches(r, query.list_name.as_deref()))
                    .collect();
                dated.extend(undated);
            }
            dated
        } else {
            mgr.fetch_incomplete_reminders().map_err(map_ek_err)?
        };

        if let Some(ref list) = query.list_name {
            items.retain(|r| calendar_matches(r, Some(list.as_str())));
        }

        let search = query.search.as_ref().map(|s| s.to_ascii_lowercase());

        let mut matched: Vec<_> = items
            .into_iter()
            .filter(|item| {
                if let Some(ref q) = search {
                    if !item.title.to_ascii_lowercase().contains(q) {
                        return false;
                    }
                }
                matches_window_item(item, query)
            })
            .collect();

        matched.truncate(query.limit as usize);

        if matched.is_empty() {
            return Ok(empty_message(query));
        }

        let mut out = String::new();
        for item in matched {
            let status = if item.completed { " ✓" } else { "" };
            if let Some(d) = item.due_date {
                out.push_str(&format!(
                    "- {}{} (due: {})\n",
                    item.title,
                    status,
                    d.format("%Y-%m-%d %H:%M")
                ));
            } else {
                out.push_str(&format!("- {}{} (no due date)\n", item.title, status));
            }
        }
        Ok(out.trim_end().to_string())
    });

    match eventkit_result {
        Ok(msg) => Ok(msg),
        Err(e) if is_auth_failure(&e) => list_reminders_applescript(query),
        Err(e) => Err(e),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn list_reminders(_query: &ListQuery) -> Result<String, String> {
    Err("reminders are only supported on macOS".into())
}

#[cfg(target_os = "macos")]
pub fn complete_reminder(
    title: &str,
    match_mode: &str,
    list_name: Option<&str>,
) -> Result<String, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("title must not be empty".into());
    }

    let mode = match match_mode.trim().to_ascii_lowercase().as_str() {
        "" | "exact" => "exact",
        "contains" | "partial" | "fuzzy" => "contains",
        other => {
            return Err(format!(
                "unknown match \"{other}\"; use exact or contains"
            ));
        }
    };

    with_reminders(|mgr| {
        mgr.ensure_authorized().map_err(map_ek_err)?;

        let items = mgr.fetch_incomplete_reminders().map_err(map_ek_err)?;

        let needle = title.to_ascii_lowercase();
        let found = items.into_iter().find(|item| {
            if !calendar_matches(item, list_name) {
                return false;
            }
            let hay = item.title.to_ascii_lowercase();
            if mode == "exact" {
                hay == needle
            } else {
                hay.contains(&needle)
            }
        });

        let Some(item) = found else {
            return Err(format!(
                "There is no incomplete reminder matching \"{title}\"."
            ));
        };

        let done = mgr
            .complete_reminder(&item.identifier)
            .map_err(map_ek_err)?;
        Ok(format!("Completed reminder \"{}\"", done.title))
    })
    .or_else(|e| {
        if is_auth_failure(&e) {
            complete_reminder_applescript(title, mode)
        } else {
            Err(e)
        }
    })
}

#[cfg(not(target_os = "macos"))]
pub fn complete_reminder(
    _title: &str,
    _match_mode: &str,
    _list_name: Option<&str>,
) -> Result<String, String> {
    Err("reminders are only supported on macOS".into())
}

/// Warm EventKit permission on the main thread (do not background this).
#[cfg(target_os = "macos")]
pub fn prewarm_eventkit() {
    let _ = with_reminders(|mgr| {
        // Only request auth — don't fetch the whole list on startup (slow).
        mgr.ensure_authorized().map_err(map_ek_err)?;
        Ok(())
    });
}

#[cfg(not(target_os = "macos"))]
pub fn prewarm_eventkit() {}

// --- shared helpers (all platforms for tests) --------------------------------

fn empty_message(query: &ListQuery) -> String {
    if let Some(ref q) = query.search {
        return format!("No reminders matching \"{q}\".");
    }
    if query.window_start.is_some() || query.window_end.is_some() {
        return "No reminders in that date range.".into();
    }
    "No incomplete reminders.".into()
}

#[cfg(target_os = "macos")]
fn calendar_matches(item: &eventkit::ReminderItem, list_name: Option<&str>) -> bool {
    match list_name {
        None => true,
        Some(want) => item
            .calendar_title
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case(want)),
    }
}

#[cfg(target_os = "macos")]
fn matches_window_item(item: &eventkit::ReminderItem, query: &ListQuery) -> bool {
    let has_window = query.window_start.is_some() || query.window_end.is_some();
    if !has_window {
        return true;
    }

    match item.due_date {
        None => query.include_undated,
        Some(dt) => {
            let ord = date_to_ord(dt.date_naive());
            let after_start = match query.window_start {
                Some(start) => ord >= start,
                None => true,
            };
            let before_end = match query.window_end {
                Some(end) => ord < end,
                None => true,
            };
            let in_window = after_start && before_end;
            let overdue = match query.window_start {
                Some(start) => query.include_overdue && ord < start,
                None => false,
            };
            in_window || overdue
        }
    }
}

#[cfg(target_os = "macos")]
fn with_reminders<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&eventkit::RemindersManager) -> Result<T, String>,
{
    thread_local! {
        static TLS_MGR: std::cell::RefCell<Option<eventkit::RemindersManager>> =
            const { std::cell::RefCell::new(None) };
    }
    TLS_MGR.with(|slot| {
        let mut borrow = slot.borrow_mut();
        if borrow.is_none() {
            *borrow = Some(eventkit::RemindersManager::new());
        }
        f(borrow.as_ref().unwrap())
    })
}

#[cfg(target_os = "macos")]
fn map_ek_err(e: eventkit::EventKitError) -> String {
    match e {
        eventkit::EventKitError::AuthorizationDenied => {
            "Reminders access denied. In System Settings → Privacy & Security → Reminders, enable \
             “Hey Martha” (or “hey-martha”). If nothing is listed, EventKit can’t register this \
             `tauri dev` binary — run a built .app once, or Martha will fall back to AppleScript."
                .into()
        }
        eventkit::EventKitError::AuthorizationRestricted => {
            "Reminders access is restricted on this device.".into()
        }
        other => format!("{other}"),
    }
}

/// True when EventKit failed because TCC blocked us (so AppleScript can still work).
#[cfg(target_os = "macos")]
fn is_auth_failure(msg: &str) -> bool {
    msg.contains("access denied")
        || msg.contains("AuthorizationDenied")
        || msg.contains("Reminders access denied")
}

#[cfg(target_os = "macos")]
fn create_reminder_applescript(
    title: &str,
    due_iso: Option<&str>,
    notes: Option<&str>,
) -> Result<String, String> {
    use std::process::Command;

    let escaped = title.replace('\\', "\\\\").replace('"', "\\\"");
    let notes_prop = match notes.map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => {
            let e = n.replace('\\', "\\\\").replace('"', "\\\"");
            format!(", body:\"{e}\"")
        }
        None => String::new(),
    };

    let script = match due_iso.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => {
            let due = parse_due(raw)?;
            format!(
                r#"tell application "Reminders"
  set dueDate to current date
  set year of dueDate to {year}
  set month of dueDate to {month}
  set day of dueDate to {day}
  set hours of dueDate to {hour}
  set minutes of dueDate to {minute}
  set seconds of dueDate to {second}
  make new reminder with properties {{name:"{name}"{notes}, due date:dueDate}}
end tell"#,
                year = due.0,
                month = month_name(due.1),
                day = due.2,
                hour = due.3,
                minute = due.4,
                second = due.5,
                name = escaped,
                notes = notes_prop,
            )
        }
        None => format!(
            r#"tell application "Reminders"
  make new reminder with properties {{name:"{name}"{notes}}}
end tell"#,
            name = escaped,
            notes = notes_prop,
        ),
    };

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("failed to run osascript: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Reminders AppleScript error: {}",
            stderr.trim()
        ));
    }

    match due_iso.map(str::trim).filter(|s| !s.is_empty()) {
        Some(t) => Ok(format!("Created reminder \"{title}\" due {t}")),
        None => Ok(format!("Created reminder \"{title}\"")),
    }
}

#[cfg(target_os = "macos")]
fn list_reminders_applescript(query: &ListQuery) -> Result<String, String> {
    use std::process::Command;

    let limit = query.limit.clamp(1, 100);
    let script = format!(
        r#"tell application "Reminders"
  set output to ""
  set n to 0
  set theReminders to reminders whose completed is false
  repeat with r in theReminders
    if n ≥ {limit} then exit repeat
    set rName to name of r
    set rDue to ""
    try
      if due date of r is not missing value then
        set rDue to (due date of r) as string
      end if
    end try
    if rDue is "" then
      set output to output & "- " & rName & linefeed
    else
      set output to output & "- " & rName & " (due: " & rDue & ")" & linefeed
    end if
    set n to n + 1
  end repeat
  if output is "" then
    return "No incomplete reminders."
  end if
  return output
end tell"#,
        limit = limit
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("failed to run osascript: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Reminders AppleScript error: {}",
            stderr.trim()
        ));
    }

    let mut lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if let Some(ref q) = query.search {
        let needle = q.to_ascii_lowercase();
        lines.retain(|l| l.to_ascii_lowercase().contains(&needle));
    }

    if lines.is_empty() {
        Ok(empty_message(query))
    } else {
        Ok(lines.join("\n"))
    }
}

#[cfg(target_os = "macos")]
fn complete_reminder_applescript(title: &str, mode: &str) -> Result<String, String> {
    use std::process::Command;

    let escaped = title.replace('\\', "\\\\").replace('"', "\\\"");
    let whose = if mode == "exact" {
        format!(r#"name is "{escaped}" and completed is false"#)
    } else {
        format!(r#"name contains "{escaped}" and completed is false"#)
    };

    let script = format!(
        r#"tell application "Reminders"
  set matches to (reminders whose {whose})
  if (count of matches) is 0 then
    return "NOT_FOUND"
  end if
  set completed of item 1 of matches to true
  set doneName to name of item 1 of matches
  return "OK:" & doneName
end tell"#
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("failed to run osascript: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Reminders AppleScript error: {}",
            stderr.trim()
        ));
    }

    let trimmed = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if trimmed == "NOT_FOUND" {
        return Err(format!(
            "There is no incomplete reminder matching \"{title}\"."
        ));
    }
    if let Some(name) = trimmed.strip_prefix("OK:") {
        return Ok(format!("Completed reminder \"{}\"", name.trim()));
    }
    Err(format!("Unexpected Reminders response: {trimmed}"))
}

#[cfg(target_os = "macos")]
fn parse_due(raw: &str) -> Result<(u32, u32, u32, u32, u32, u32), String> {
    let dt = parse_due_datetime(raw)?;
    Ok((
        dt.year() as u32,
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second(),
    ))
}

#[cfg(target_os = "macos")]
fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "January",
    }
}

/// Open macOS Settings to the Reminders privacy pane (so the user can grant access).
#[cfg(target_os = "macos")]
pub fn open_reminders_privacy_settings() -> Result<String, String> {
    use std::process::Command;
    let status = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Reminders")
        .status()
        .map_err(|e| format!("failed to open System Settings: {e}"))?;
    if status.success() {
        Ok("Opened System Settings → Privacy → Reminders. Enable Hey Martha / hey-martha if listed."
            .into())
    } else {
        Err("Could not open System Settings.".into())
    }
}

#[cfg(not(target_os = "macos"))]
pub fn open_reminders_privacy_settings() -> Result<String, String> {
    Err("only available on macOS".into())
}

#[cfg(target_os = "macos")]
fn ord_to_local_midnight(ord: u32) -> Result<DateTime<Local>, String> {
    let (y, m, d) = ord_to_ymd(ord);
    Local
        .with_ymd_and_hms(y as i32, m, d, 0, 0, 0)
        .single()
        .ok_or_else(|| format!("invalid date ordinal {ord}"))
}

fn parse_due_datetime(raw: &str) -> Result<DateTime<Local>, String> {
    let s = raw.trim().trim_end_matches('Z').trim_end_matches('z');
    let (date_part, time_part) = if let Some((d, t)) = s.split_once('T') {
        (d, Some(t))
    } else if let Some((d, t)) = s.split_once(' ') {
        (d, Some(t))
    } else {
        (s, None)
    };

    let (y, m, d) = parse_ymd(date_part)?;
    let (hour, minute, second) = match time_part {
        None => (9u32, 0, 0),
        Some(t) => {
            let tb: Vec<&str> = t.split(':').collect();
            if tb.len() < 2 || tb.len() > 3 {
                return Err(format!(
                    "invalid time \"{raw}\"; use HH:MM or HH:MM:SS"
                ));
            }
            let hour: u32 = tb[0]
                .parse()
                .map_err(|_| format!("invalid hour in \"{raw}\""))?;
            let minute: u32 = tb[1]
                .parse()
                .map_err(|_| format!("invalid minute in \"{raw}\""))?;
            let second: u32 = if tb.len() == 3 {
                tb[2]
                    .parse()
                    .map_err(|_| format!("invalid second in \"{raw}\""))?
            } else {
                0
            };
            (hour, minute, second)
        }
    };

    let naive = NaiveDate::from_ymd_opt(y as i32, m, d)
        .and_then(|date| date.and_hms_opt(hour, minute, second))
        .ok_or_else(|| format!("invalid due date \"{raw}\""))?;
    Ok(Local.from_local_datetime(&naive).single().ok_or_else(|| {
        format!("invalid local due date \"{raw}\"")
    })?)
}

fn parse_ymd(raw: &str) -> Result<(u32, u32, u32), String> {
    let date_part = raw
        .split('T')
        .next()
        .unwrap_or(raw)
        .split(' ')
        .next()
        .unwrap_or(raw)
        .trim();
    let bits: Vec<&str> = date_part.split('-').collect();
    if bits.len() != 3 {
        return Err(format!("invalid date \"{raw}\"; use YYYY-MM-DD"));
    }
    let year: u32 = bits[0]
        .parse()
        .map_err(|_| format!("invalid year in \"{raw}\""))?;
    let month: u32 = bits[1]
        .parse()
        .map_err(|_| format!("invalid month in \"{raw}\""))?;
    let day: u32 = bits[2]
        .parse()
        .map_err(|_| format!("invalid day in \"{raw}\""))?;
    if !(1..=12).contains(&month) {
        return Err(format!("month must be 1–12, got {month}"));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("day must be 1–31, got {day}"));
    }
    Ok((year, month, day))
}

fn ymd_ord(y: u32, m: u32, d: u32) -> u32 {
    y * 512 + m * 32 + d
}

fn ord_to_ymd(ord: u32) -> (u32, u32, u32) {
    let y = ord / 512;
    let rem = ord % 512;
    let m = rem / 32;
    let d = rem % 32;
    (y, m, d)
}

fn date_to_ord(date: NaiveDate) -> u32 {
    ymd_ord(date.year() as u32, date.month(), date.day())
}

fn today_ymd() -> Result<(u32, u32, u32), String> {
    let now = Local::now().date_naive();
    Ok((now.year() as u32, now.month(), now.day()))
}

fn ymd_plus_days(y: u32, m: u32, d: u32, days: i32) -> Result<(u32, u32, u32), String> {
    use std::process::Command;
    let base = format!("{y:04}-{m:02}-{d:02}");
    let output = Command::new("date")
        .args([
            "-j",
            &format!("-v{days:+}d"),
            "-f",
            "%Y-%m-%d",
            &base,
            "+%Y-%m-%d",
        ])
        .output()
        .map_err(|e| format!("failed to compute date offset: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("failed to compute date offset: {stderr}"));
    }
    parse_ymd(String::from_utf8_lossy(&output.stdout).trim())
}

fn ymd_ord_plus_days(y: u32, m: u32, d: u32, days: i32) -> Result<u32, String> {
    let (yy, mm, dd) = ymd_plus_days(y, m, d, days)?;
    Ok(ymd_ord(yy, mm, dd))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_iso_datetime() {
        let dt = parse_due_datetime("2026-07-20T15:30").unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-07-20 15:30");
    }

    #[test]
    fn parses_date_only_default_time() {
        let dt = parse_due_datetime("2026-07-20").unwrap();
        assert_eq!(dt.format("%H:%M").to_string(), "09:00");
    }

    #[test]
    fn rejects_bad_format() {
        assert!(parse_due_datetime("tomorrow").is_err());
    }

    #[test]
    fn query_days_ahead() {
        let q = list_query_from_json(&json!({ "days_ahead": 3 })).unwrap();
        assert!(q.window_start.is_some());
        assert!(q.window_end.is_some());
        assert!(!q.include_undated);
    }

    #[test]
    fn query_start_end() {
        let q = list_query_from_json(&json!({
            "start": "2026-07-19",
            "end": "2026-07-21"
        }))
        .unwrap();
        assert_eq!(q.window_start, Some(ymd_ord(2026, 7, 19)));
        assert_eq!(q.window_end, Some(ymd_ord(2026, 7, 22)));
    }

    #[test]
    fn query_all_includes_undated() {
        let q = list_query_from_json(&json!({})).unwrap();
        assert!(q.include_undated);
        assert!(q.window_start.is_none());
    }
}
