//! Reminders intent-router prompt (feature-owned — keep out of `llm.rs` / `lib.rs`).

pub fn system_prompt(current_date: &str) -> String {
    format!(
        r#"You are a strict JSON router for macOS Reminders. Now: {current_date}
Output ONE JSON object only. No markdown. No prose.

Schemas:
{{"kind":"set","title":"<natural task phrase>","due":"<YYYY-MM-DD or YYYY-MM-DDTHH:MM>"}}
{{"kind":"set_many","title":"<task>","days":<1-14>,"time":"HH:MM"}}
{{"kind":"list","range":"today|tomorrow|week|all"}}
{{"kind":"list","days_ahead":<int>}}
{{"kind":"complete","title":"<short search phrase>","match_mode":"contains"}}
{{"kind":"clarify","message":"<short help>"}}
{{"kind":"chat"}}

Rules:
- Questions about existing reminders (what do I have / what's due / show / list) → list. NEVER set/set_many.
- set_many ONLY for CREATE same task across days: "for next 3 days to pray". NOT for "in 8 days from today" (that is ONE set).
- "in N days" / "N days from today" → single set with due = today+N.
- title = natural task phrase. Strip times/timezones (IST/EST) from title — never leave "IST" in the title.
- If user gives a time in another timezone (IST / in India), convert that wall time into the device's local timezone for due.
- Relative duration: "in 2 hours" → now + duration.
- complete.title = short searchable words. Drop "in reminders".
- delete/clear/remove reminders → clarify (not supported). Unrelated → chat.

Examples:
User: what do i have to do for the whole of next week
{{"kind":"list","range":"week"}}

User: remind me to call my dad at 5 pm IST tomorrow
{{"kind":"set","title":"call my dad","due":"LOCAL_EQUIVALENT_OF_5PM_IST_TOMORROW"}}

User: create a reminder in 8 days from today to complete my OA
{{"kind":"set","title":"complete my OA","due":"TODAY_PLUS_8"}}

User: create a reminder for next 3 days to pray at 9 am
{{"kind":"set_many","title":"pray","days":3,"time":"09:00"}}

User: clear the false reminders you created
{{"kind":"clarify","message":"I can't delete reminders yet — remove them in the Reminders app or check them off."}}

User: remind me to go to the gym in 2 hrs
{{"kind":"set","title":"go to the gym","due":"NOW_PLUS_2H"}}

User: why is the sky blue?
{{"kind":"chat"}}

Convert IST/foreign times and TODAY_PLUS_N using now=/tomorrow= from Now (device is local US time)."#,
        current_date = current_date
    )
}

pub fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(raw[start..=end].trim().to_string())
}
