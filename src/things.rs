use std::process::Command;

use chrono::{DateTime, Datelike, Local, Timelike};
use serde::de::DeserializeOwned;

use crate::{
    error::ThingError,
    model::{Thing, ThingList},
};

pub struct ThingsApp;

#[derive(Debug, Clone)]
pub struct CreateThingInput {
    pub list_name: String,
    pub name: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateThingInput {
    pub name: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy)]
pub struct ScheduleTarget {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl ScheduleTarget {
    pub fn from_local(value: DateTime<Local>) -> Self {
        Self {
            year: value.year(),
            month: value.month(),
            day: value.day(),
            hour: value.hour(),
            minute: value.minute(),
            second: value.second(),
        }
    }
}

impl ThingsApp {
    pub fn new() -> Self {
        Self
    }

    pub fn lists(&self) -> Result<Vec<ThingList>, ThingError> {
        self.run_json_script(LISTS_BODY, &[])
    }

    pub fn list_things(&self) -> Result<Vec<Thing>, ThingError> {
        self.run_json_script(LIST_THINGS_BODY, &[])
    }

    pub fn list_things_in_list(&self, list_name: &str) -> Result<Vec<Thing>, ThingError> {
        self.run_json_script(LIST_THINGS_IN_LIST_BODY, &[list_name])
    }

    pub fn find_by_id(&self, task_id: &str) -> Result<Thing, ThingError> {
        self.run_json_script(FIND_THING_BY_ID_BODY, &[task_id])
    }

    pub fn find_exact_name_matches(&self, name: &str) -> Result<Vec<Thing>, ThingError> {
        self.run_json_script(EXACT_NAME_MATCHES_BODY, &[name])
    }

    pub fn find_case_insensitive_name_matches(&self, name: &str) -> Result<Vec<Thing>, ThingError> {
        self.run_json_script(CASE_INSENSITIVE_NAME_MATCHES_BODY, &[name])
    }

    pub fn create(&self, input: CreateThingInput) -> Result<Thing, ThingError> {
        let notes = input.notes.unwrap_or_default();
        let tags = input.tags.join(", ");
        let id = self.run_text_script(
            CREATE_THING_BODY,
            &[&input.list_name, &input.name, &notes, &tags],
        )?;
        self.get_by_id(&id)
    }

    pub fn update(&self, task_id: &str, input: UpdateThingInput) -> Result<Thing, ThingError> {
        let name_was_provided = input.name.is_some();
        let notes_were_provided = input.notes.is_some();
        let status_was_provided = input.status.is_some();
        let tags_were_provided = input.tags.is_some();
        let name = input.name.unwrap_or_default();
        let notes = input.notes.unwrap_or_default();
        let status = input.status.unwrap_or_default();
        let tags = input.tags.unwrap_or_default().join(", ");

        self.run_text_script(
            UPDATE_THING_BODY,
            &[
                task_id,
                &name,
                flag(name_was_provided),
                &notes,
                flag(notes_were_provided),
                &status,
                flag(status_was_provided),
                &tags,
                flag(tags_were_provided),
            ],
        )?;
        self.get_by_id(task_id)
    }

    pub fn complete(&self, task_id: &str) -> Result<Thing, ThingError> {
        self.run_text_script(COMPLETE_THING_BODY, &[task_id])?;
        self.get_by_id(task_id)
    }

    pub fn move_to(&self, task_id: &str, list_name: &str) -> Result<Thing, ThingError> {
        self.run_text_script(MOVE_THING_BODY, &[task_id, list_name])?;
        self.get_by_id(task_id)
    }

    pub fn schedule(&self, task_id: &str, when: ScheduleTarget) -> Result<Thing, ThingError> {
        self.run_text_script(
            SCHEDULE_THING_BODY,
            &[
                task_id,
                &when.year.to_string(),
                &when.month.to_string(),
                &when.day.to_string(),
                &when.hour.to_string(),
                &when.minute.to_string(),
                &when.second.to_string(),
            ],
        )?;
        self.get_by_id(task_id)
    }

    pub fn delete(&self, task_id: &str) -> Result<(), ThingError> {
        self.run_text_script(DELETE_THING_BODY, &[task_id])?;
        Ok(())
    }

    pub fn open(&self, task_id: &str) -> Result<(), ThingError> {
        self.run_text_script(OPEN_THING_BODY, &[task_id])?;
        Ok(())
    }

    fn get_by_id(&self, task_id: &str) -> Result<Thing, ThingError> {
        self.find_by_id(task_id)
    }

    fn run_json_script<T: DeserializeOwned>(
        &self,
        body: &str,
        args: &[&str],
    ) -> Result<T, ThingError> {
        let output = self.run_osascript(body, args)?;
        serde_json::from_str(&output).map_err(|source| ThingError::Json {
            path: "<osascript>".into(),
            source,
        })
    }

    fn run_text_script(&self, body: &str, args: &[&str]) -> Result<String, ThingError> {
        self.run_osascript(body, args)
    }

    fn run_osascript(&self, body: &str, args: &[&str]) -> Result<String, ThingError> {
        let script = format!("{SHARED_HELPERS}\n{body}");
        let mut command = Command::new("/usr/bin/osascript");
        command.arg("-e").arg(script).arg("--");
        for arg in args {
            command.arg(arg);
        }

        let output = command.output().map_err(|source| ThingError::Io {
            path: "/usr/bin/osascript".into(),
            source,
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let details = match (!stderr.is_empty(), !stdout.is_empty()) {
                (true, true) => format!("{stderr} | stdout: {stdout}"),
                (true, false) => stderr,
                (false, true) => stdout,
                (false, false) => "unknown osascript failure".to_string(),
            };
            return Err(ThingError::Automation(details));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

fn flag(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

const SHARED_HELPERS: &str = r#"
use framework "Foundation"
use scripting additions

on iso_string(date_value)
  if date_value is missing value then return missing value
  set formatter to current application's NSISO8601DateFormatter's alloc()'s init()
  formatter's setFormatOptions:(current application's NSISO8601DateFormatWithInternetDateTime)
  return (formatter's stringFromDate:date_value) as text
end iso_string

on trim_text(value_text)
  set ns_text to current application's NSString's stringWithString:(value_text as text)
  set trimmed to ns_text's stringByTrimmingCharactersInSet:(current application's NSCharacterSet's whitespaceAndNewlineCharacterSet())
  return trimmed as text
end trim_text

on tags_array(raw_tags)
  set results to current application's NSMutableArray's alloc()'s init()
  if raw_tags is missing value then return results
  set raw_text to raw_tags as text
  if raw_text is "" then return results
  set previous_delimiters to AppleScript's text item delimiters
  set AppleScript's text item delimiters to ","
  set pieces to text items of raw_text
  set AppleScript's text item delimiters to previous_delimiters
  repeat with piece in pieces
    set trimmed_piece to my trim_text(piece)
    if trimmed_piece is not "" then
      (results's addObject:trimmed_piece)
    end if
  end repeat
  return results
end tags_array

on maybe_set_text(payload, key_name, value_text)
  if value_text is missing value then return
  set normalized to value_text as text
  if normalized is "" then return
  (payload's setObject:normalized forKey:key_name)
end maybe_set_text

on maybe_set_date(payload, key_name, value_date)
  set iso_text to my iso_string(value_date)
  if iso_text is missing value then return
  (payload's setObject:iso_text forKey:key_name)
end maybe_set_date

on lowercase_text(value_text)
  set ns_text to current application's NSString's stringWithString:(value_text as text)
  return (ns_text's lowercaseString()) as text
end lowercase_text

on find_task_ref(task_id)
  tell application "Things3"
    try
      return first to do whose id is task_id
    end try
    try
      return first to do of list "Logbook" whose id is task_id
    end try
  end tell
  error "Things task not found: " & task_id
end find_task_ref

on thing_record(task_ref, container_name, container_kind)
  set payload to current application's NSMutableDictionary's alloc()'s init()
  tell application "Things3"
    set task_id to id of task_ref
    set task_name to name of task_ref
    set task_notes to notes of task_ref
    set task_status to (status of task_ref) as text
    set task_tag_names to tag names of task_ref
  end tell
  (payload's setObject:task_id forKey:"id")
  (payload's setObject:task_name forKey:"name")
  my maybe_set_text(payload, "notes", task_notes)
  (payload's setObject:task_status forKey:"status")
  (payload's setObject:(my tags_array(task_tag_names)) forKey:"tags")
  my maybe_set_text(payload, "container_name", container_name)
  my maybe_set_text(payload, "container_kind", container_kind)
  try
    tell application "Things3" to set project_name to name of (project of task_ref)
    my maybe_set_text(payload, "project", project_name)
  end try
  try
    tell application "Things3" to set area_name to name of (area of task_ref)
    my maybe_set_text(payload, "area", area_name)
  end try
  try
    tell application "Things3" to set contact_name to name of (contact of task_ref)
    my maybe_set_text(payload, "contact", contact_name)
  end try
  tell application "Things3"
    my maybe_set_date(payload, "due_date", (due date of task_ref))
    my maybe_set_date(payload, "activation_date", (activation date of task_ref))
    my maybe_set_date(payload, "completion_date", (completion date of task_ref))
    my maybe_set_date(payload, "cancellation_date", (cancellation date of task_ref))
    my maybe_set_date(payload, "creation_date", (creation date of task_ref))
    my maybe_set_date(payload, "modification_date", (modification date of task_ref))
  end tell
  return payload
end thing_record

on infer_container_name(task_ref)
  try
    tell application "Things3" to return name of (project of task_ref)
  end try
  try
    tell application "Things3" to return name of (area of task_ref)
  end try
  try
    tell application "Things3" to return name of (contact of task_ref)
  end try
  return missing value
end infer_container_name

on infer_container_kind(task_ref)
  try
    tell application "Things3" to name of (project of task_ref)
    return "project"
  end try
  try
    tell application "Things3" to name of (area of task_ref)
    return "area"
  end try
  try
    tell application "Things3" to name of (contact of task_ref)
    return "contact"
  end try
  return missing value
end infer_container_kind

on active_thing_record(task_ref)
  set container_name to my infer_container_name(task_ref)
  set container_kind to my infer_container_kind(task_ref)
  return my thing_record(task_ref, container_name, container_kind)
end active_thing_record

on json_string(payload)
  set {json_data, json_error} to current application's NSJSONSerialization's dataWithJSONObject:payload options:0 |error|:(reference)
  if json_data is missing value then error (json_error's localizedDescription() as text)
  return (current application's NSString's alloc()'s initWithData:json_data encoding:(current application's NSUTF8StringEncoding)) as text
end json_string
"#;

const LISTS_BODY: &str = r#"
on run argv
  set payload to current application's NSMutableArray's alloc()'s init()
  tell application "Things3"
    repeat with list_ref in lists
      set item_payload to current application's NSMutableDictionary's alloc()'s init()
      (item_payload's setObject:(id of list_ref) forKey:"id")
      (item_payload's setObject:(name of list_ref) forKey:"name")
      (payload's addObject:item_payload)
    end repeat
  end tell
  return my json_string(payload)
end run
"#;

const LIST_THINGS_BODY: &str = r#"
on run argv
  set collector_ref to current application's NSMutableArray's alloc()'s init()
  tell application "Things3"
    set task_refs to to dos
    repeat with task_item in task_refs
      set task_ref to contents of task_item
      (collector_ref's addObject:(my active_thing_record(task_ref)))
    end repeat
  end tell
  return my json_string(collector_ref)
end run
"#;

const LIST_THINGS_IN_LIST_BODY: &str = r#"
on run argv
  set list_name to item 1 of argv
  set collector_ref to current application's NSMutableArray's alloc()'s init()
  tell application "Things3"
    set task_refs to to dos of list list_name
    repeat with task_item in task_refs
      set task_ref to contents of task_item
      (collector_ref's addObject:(my thing_record(task_ref, list_name, "list")))
    end repeat
  end tell
  return my json_string(collector_ref)
end run
"#;

const CREATE_THING_BODY: &str = r#"
on run argv
  set list_name to item 1 of argv
  set task_name to item 2 of argv
  set task_notes to item 3 of argv
  set task_tags to item 4 of argv
  tell application "Things3"
    set target_list to list list_name
    set new_task to make new to do at target_list with properties {name:task_name, notes:task_notes}
    if task_tags is not "" then set tag names of new_task to task_tags
    return id of new_task
  end tell
end run
"#;

const FIND_THING_BY_ID_BODY: &str = r#"
on run argv
  set task_id to item 1 of argv
  tell application "Things3"
    try
      set task_ref to first to do whose id is task_id
      return my json_string(my active_thing_record(task_ref))
    end try
    try
      set task_ref to first to do of list "Logbook" whose id is task_id
      return my json_string(my thing_record(task_ref, "Logbook", "list"))
    end try
  end tell
  error "Things task not found: " & task_id
end run
"#;

const EXACT_NAME_MATCHES_BODY: &str = r#"
on run argv
  set task_name to item 1 of argv
  set collector_ref to current application's NSMutableArray's alloc()'s init()
  tell application "Things3"
    repeat with task_item in (to dos whose name is task_name)
      (collector_ref's addObject:(my active_thing_record(contents of task_item)))
    end repeat
    repeat with task_item in (to dos of list "Logbook" whose name is task_name)
      (collector_ref's addObject:(my thing_record(contents of task_item, "Logbook", "list")))
    end repeat
  end tell
  return my json_string(collector_ref)
end run
"#;

const CASE_INSENSITIVE_NAME_MATCHES_BODY: &str = r#"
on append_casefold_matches(task_refs, target_lower, container_name, collector_ref)
  repeat with task_item in task_refs
    set task_ref to contents of task_item
    tell application "Things3" to set task_name to name of task_ref
    if (my lowercase_text(task_name)) is target_lower then
      if container_name is missing value then
        (collector_ref's addObject:(my active_thing_record(task_ref)))
      else
        (collector_ref's addObject:(my thing_record(task_ref, container_name, "list")))
      end if
    end if
  end repeat
end append_casefold_matches

on run argv
  set target_lower to my lowercase_text(item 1 of argv)
  set collector_ref to current application's NSMutableArray's alloc()'s init()
  tell application "Things3"
    my append_casefold_matches(to dos, target_lower, missing value, collector_ref)
    my append_casefold_matches((to dos of list "Logbook"), target_lower, "Logbook", collector_ref)
  end tell
  return my json_string(collector_ref)
end run
"#;

const UPDATE_THING_BODY: &str = r#"
on run argv
  tell application "Things3"
    set task_ref to my find_task_ref(item 1 of argv)
    if item 3 of argv is "1" then set name of task_ref to item 2 of argv
    if item 5 of argv is "1" then set notes of task_ref to item 4 of argv
    if item 7 of argv is "1" then
      set next_status to item 6 of argv
      if next_status is "open" then
        set status of task_ref to open
      else if next_status is "completed" then
        set status of task_ref to completed
      else if next_status is "canceled" then
        set status of task_ref to canceled
      end if
    end if
    if item 9 of argv is "1" then set tag names of task_ref to item 8 of argv
    return id of task_ref
  end tell
end run
"#;

const COMPLETE_THING_BODY: &str = r#"
on run argv
  tell application "Things3"
    set task_ref to my find_task_ref(item 1 of argv)
    set status of task_ref to completed
    return id of task_ref
  end tell
end run
"#;

const MOVE_THING_BODY: &str = r#"
on run argv
  tell application "Things3"
    set task_ref to my find_task_ref(item 1 of argv)
    set target_list to list (item 2 of argv)
    move task_ref to target_list
    return id of task_ref
  end tell
end run
"#;

const SCHEDULE_THING_BODY: &str = r#"
on run argv
  set target_date to current date
  set day of target_date to 1
  set year of target_date to ((item 2 of argv) as integer)
  set month of target_date to ((item 3 of argv) as integer)
  set day of target_date to ((item 4 of argv) as integer)
  set time of target_date to ((((item 5 of argv) as integer) * hours) + (((item 6 of argv) as integer) * minutes) + ((item 7 of argv) as integer))
  tell application "Things3"
    set task_ref to my find_task_ref(item 1 of argv)
    schedule task_ref for target_date
    return id of task_ref
  end tell
end run
"#;

const DELETE_THING_BODY: &str = r#"
on run argv
  tell application "Things3"
    try
      move (first to do of list "Logbook" whose id is item 1 of argv) to list "Trash"
      return item 1 of argv
    end try
    try
      delete first to do whose id is item 1 of argv
      return item 1 of argv
    end try
  end tell
  error "Things task not found: " & item 1 of argv
end run
"#;

const OPEN_THING_BODY: &str = r#"
on run argv
  tell application "Things3"
    set task_ref to my find_task_ref(item 1 of argv)
    show task_ref
    activate
    return id of task_ref
  end tell
end run
"#;
