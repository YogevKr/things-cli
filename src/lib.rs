mod cli;
mod error;
mod model;
mod output;
mod things;

use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
};

use chrono::{DateTime, Days, Local, NaiveDate, NaiveDateTime, TimeZone};
use clap::Parser;
use serde_json::json;

use crate::{
    cli::{Cli, Command, CreateArgs, ListArgs, MoveArgs, ScheduleArgs, UpdateArgs},
    error::ThingError,
    model::{Thing, ThingList},
    output::Output,
    things::{CreateThingInput, ScheduleTarget, ThingsApp, UpdateThingInput},
};

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let json_output = cli.json;

    match execute(cli).and_then(|output| render(&output, json_output)) {
        Ok(rendered) => {
            if !rendered.is_empty() {
                println!("{rendered}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if json_output {
                let payload = json!({
                    "error": error.to_string(),
                    "kind": error.kind(),
                    "exit_code": error.exit_code(),
                });

                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
                        "{\"error\":\"failed to serialize error\"}".to_string()
                    })
                );
            } else {
                eprintln!("error: {error}");
            }

            ExitCode::from(error.exit_code())
        }
    }
}

fn execute(cli: Cli) -> Result<Output, ThingError> {
    let app = ThingsApp::new();

    match cli.command {
        Command::Lists => Ok(Output::Lists {
            lists: app.lists()?,
        }),
        Command::List(args) => list_things(&app, args),
        Command::Get(args) => Ok(Output::Thing {
            thing: resolve_thing(&app, &args.selector)?,
        }),
        Command::Create(args) => create_thing(&app, args),
        Command::Update(args) => update_thing(&app, args),
        Command::Complete(args) => {
            let thing = resolve_thing(&app, &args.selector)?;
            Ok(Output::Thing {
                thing: app.complete(&thing.id)?,
            })
        }
        Command::Move(args) => move_thing(&app, args),
        Command::Schedule(args) => schedule_thing(&app, args),
        Command::Delete(args) => Ok(Output::Deleted {
            deleted: true,
            thing: delete_thing(&app, &args.selector)?,
        }),
        Command::Open(args) => {
            let thing = resolve_thing(&app, &args.selector)?;
            app.open(&thing.id)?;
            Ok(Output::Opened {
                opened: true,
                thing,
            })
        }
    }
}

fn list_things(app: &ThingsApp, args: ListArgs) -> Result<Output, ThingError> {
    let mut things = if let Some(list_name) = &args.list_name {
        let resolved_list = resolve_list_name(app, list_name)?;
        app.list_things_in_list(&resolved_list)?
    } else {
        app.list_things()?
    };
    things.sort_by(|left, right| {
        right
            .modification_date
            .cmp(&left.modification_date)
            .then_with(|| left.name.cmp(&right.name))
    });

    if let Some(status) = args.status {
        let status = validate_status(status)?;
        things.retain(|thing| thing.status == status);
    }

    let tags = normalize_tags(args.tags);
    if !tags.is_empty() {
        things.retain(|thing| tags.iter().all(|tag| thing.tags.contains(tag)));
    }

    if let Some(query) = args.query {
        things.retain(|thing| thing.matches_query(&query));
    }

    if let Some(limit) = args.limit {
        things.truncate(limit);
    }

    Ok(Output::Things { things })
}

fn create_thing(app: &ThingsApp, args: CreateArgs) -> Result<Output, ThingError> {
    let name = normalize_text(args.name, "name")?;
    let notes = read_text(args.notes, args.notes_file)?;
    let list_name = resolve_list_name(app, &args.list_name)?;
    let thing = app.create(CreateThingInput {
        list_name,
        name,
        notes,
        tags: normalize_tags(args.tags),
    })?;

    Ok(Output::Thing { thing })
}

fn update_thing(app: &ThingsApp, args: UpdateArgs) -> Result<Output, ThingError> {
    let thing = resolve_thing(app, &args.selector)?;
    let notes_were_provided = args.notes.is_some() || args.notes_file.is_some();
    let tags = normalize_tags(args.tags);
    let changed = args.name.is_some()
        || notes_were_provided
        || args.clear_notes
        || args.status.is_some()
        || !tags.is_empty()
        || args.clear_tags;

    if !changed {
        return Err(ThingError::InvalidInput(
            "update requires at least one field change".to_string(),
        ));
    }

    let updated = app.update(
        &thing.id,
        UpdateThingInput {
            name: args
                .name
                .map(|value| normalize_text(value, "name"))
                .transpose()?,
            notes: if args.clear_notes {
                Some(String::new())
            } else if notes_were_provided {
                Some(read_text(args.notes, args.notes_file)?.unwrap_or_default())
            } else {
                None
            },
            status: args.status.map(validate_status).transpose()?,
            tags: if args.clear_tags {
                Some(Vec::new())
            } else if !tags.is_empty() {
                Some(tags)
            } else {
                None
            },
        },
    )?;

    Ok(Output::Thing { thing: updated })
}

fn move_thing(app: &ThingsApp, args: MoveArgs) -> Result<Output, ThingError> {
    let thing = resolve_thing(app, &args.selector)?;
    let list_name = resolve_list_name(app, &args.list_name)?;
    Ok(Output::Thing {
        thing: app.move_to(&thing.id, &list_name)?,
    })
}

fn schedule_thing(app: &ThingsApp, args: ScheduleArgs) -> Result<Output, ThingError> {
    let thing = resolve_thing(app, &args.selector)?;
    let when = parse_schedule_target(&args.when)?;
    Ok(Output::Thing {
        thing: app.schedule(&thing.id, when)?,
    })
}

fn delete_thing(app: &ThingsApp, selector: &str) -> Result<Thing, ThingError> {
    let thing = resolve_thing(app, selector)?;
    app.delete(&thing.id)?;
    Ok(thing)
}

fn resolve_thing(app: &ThingsApp, selector: &str) -> Result<Thing, ThingError> {
    if selector.trim().is_empty() {
        return Err(ThingError::InvalidInput(
            "selector must not be empty".to_string(),
        ));
    }

    if let Ok(thing) = app.find_by_id(selector) {
        return Ok(thing);
    }

    let exact = app.find_exact_name_matches(selector)?;
    match exact.as_slice() {
        [thing] => return Ok(thing.clone()),
        [] => {}
        _ => {
            return Err(ThingError::Conflict(format!(
                "multiple Things tasks share the name: {selector}"
            )));
        }
    }

    let matches = app.find_case_insensitive_name_matches(selector)?;
    match matches.as_slice() {
        [thing] => Ok(thing.clone()),
        [] => Err(ThingError::NotFound(format!(
            "Things task not found: {selector}"
        ))),
        _ => Err(ThingError::Conflict(format!(
            "multiple Things tasks match selector case-insensitively: {selector}"
        ))),
    }
}

#[cfg(test)]
fn resolve_selector<'a>(things: &'a [Thing], selector: &str) -> Result<&'a Thing, ThingError> {
    if selector.trim().is_empty() {
        return Err(ThingError::InvalidInput(
            "selector must not be empty".to_string(),
        ));
    }

    if let Some(found) = things.iter().find(|thing| thing.id == selector) {
        return Ok(found);
    }

    let exact = things
        .iter()
        .filter(|thing| thing.name == selector)
        .collect::<Vec<_>>();
    match exact.as_slice() {
        [thing] => return Ok(*thing),
        [] => {}
        _ => {
            return Err(ThingError::Conflict(format!(
                "multiple Things tasks share the name: {selector}"
            )));
        }
    }

    let matches = things
        .iter()
        .filter(|thing| thing.name.eq_ignore_ascii_case(selector))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [thing] => Ok(*thing),
        [] => Err(ThingError::NotFound(format!(
            "Things task not found: {selector}"
        ))),
        _ => Err(ThingError::Conflict(format!(
            "multiple Things tasks match selector case-insensitively: {selector}"
        ))),
    }
}

fn render(output: &Output, json_output: bool) -> Result<String, ThingError> {
    if json_output {
        return serde_json::to_string_pretty(output).map_err(|source| ThingError::Json {
            path: PathBuf::from("<stdout>"),
            source,
        });
    }

    match output {
        Output::Lists { lists } => render_lists(lists),
        Output::Thing { thing } => {
            serde_json::to_string_pretty(thing).map_err(|source| ThingError::Json {
                path: PathBuf::from("<stdout>"),
                source,
            })
        }
        Output::Things { things } => render_list(things),
        Output::Deleted { thing, .. } => Ok(format!("deleted {} ({})", thing.name, thing.id)),
        Output::Opened { thing, .. } => Ok(format!("opened {} ({})", thing.name, thing.id)),
    }
}

fn render_lists(lists: &[ThingList]) -> Result<String, ThingError> {
    if lists.is_empty() {
        return Ok("no lists found".to_string());
    }

    let mut lines = vec!["ID\tNAME".to_string()];
    lines.extend(
        lists
            .iter()
            .map(|list| format!("{}\t{}", list.id, list.name)),
    );
    Ok(lines.join("\n"))
}

fn render_list(things: &[Thing]) -> Result<String, ThingError> {
    if things.is_empty() {
        return Ok("no Things tasks found".to_string());
    }

    let mut lines = vec!["ID\tNAME\tSTATUS\tLIST\tTAGS".to_string()];
    lines.extend(things.iter().map(|thing| {
        format!(
            "{}\t{}\t{}\t{}\t{}",
            thing.id,
            thing.name,
            thing.status,
            thing.container_name.as_deref().unwrap_or("-"),
            thing.tags.join(",")
        )
    }));
    Ok(lines.join("\n"))
}

fn validate_status(status: String) -> Result<String, ThingError> {
    let normalized = normalize_text(status, "status")?.to_ascii_lowercase();
    match normalized.as_str() {
        "open" | "completed" | "canceled" => Ok(normalized),
        _ => Err(ThingError::InvalidInput(format!(
            "invalid status: {normalized}. expected open, completed, or canceled"
        ))),
    }
}

fn normalize_text(value: String, field: &str) -> Result<String, ThingError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ThingError::InvalidInput(format!(
            "{field} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut tags = tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    tags
}

fn read_text(
    value: Option<String>,
    value_file: Option<String>,
) -> Result<Option<String>, ThingError> {
    match (value, value_file) {
        (Some(value), None) => Ok(Some(value)),
        (None, Some(path)) if path == "-" => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|source| ThingError::Io {
                    path: PathBuf::from("<stdin>"),
                    source,
                })?;
            Ok(Some(buffer))
        }
        (None, Some(path)) => {
            let path_buf = PathBuf::from(path);
            let value = fs::read_to_string(&path_buf).map_err(|source| ThingError::Io {
                path: path_buf.clone(),
                source,
            })?;
            Ok(Some(value))
        }
        (None, None) => Ok(None),
        (Some(_), Some(_)) => Err(ThingError::InvalidInput(
            "value and value-file cannot be used together".to_string(),
        )),
    }
}

fn resolve_list_name(app: &ThingsApp, requested: &str) -> Result<String, ThingError> {
    let requested = normalize_text(requested.to_string(), "list")?;
    let lists = app.lists()?;

    if let Some(list) = lists.iter().find(|list| list.name == requested) {
        return Ok(list.name.clone());
    }

    let matches = lists
        .iter()
        .filter(|list| list.name.eq_ignore_ascii_case(&requested))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [list] => Ok(list.name.clone()),
        [] => Err(ThingError::NotFound(format!(
            "Things list not found: {requested}"
        ))),
        _ => Err(ThingError::Conflict(format!(
            "multiple Things lists matched: {requested}"
        ))),
    }
}

fn parse_schedule_target(raw: &str) -> Result<ScheduleTarget, ThingError> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Err(ThingError::InvalidInput(
            "schedule target must not be empty".to_string(),
        ));
    }

    let local_now = Local::now();
    let parsed = if normalized.eq_ignore_ascii_case("today") {
        local_now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight")
            .and_local_timezone(Local)
            .single()
            .expect("local midnight")
    } else if normalized.eq_ignore_ascii_case("tomorrow") {
        local_now
            .date_naive()
            .checked_add_days(Days::new(1))
            .expect("tomorrow")
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight")
            .and_local_timezone(Local)
            .single()
            .expect("local midnight")
    } else if let Ok(value) = DateTime::parse_from_rfc3339(normalized) {
        value.with_timezone(&Local)
    } else if let Ok(value) = NaiveDateTime::parse_from_str(normalized, "%Y-%m-%dT%H:%M") {
        Local.from_local_datetime(&value).single().ok_or_else(|| {
            ThingError::InvalidInput(format!("ambiguous local datetime: {normalized}"))
        })?
    } else if let Ok(value) = NaiveDateTime::parse_from_str(normalized, "%Y-%m-%d %H:%M") {
        Local.from_local_datetime(&value).single().ok_or_else(|| {
            ThingError::InvalidInput(format!("ambiguous local datetime: {normalized}"))
        })?
    } else if let Ok(value) = NaiveDate::parse_from_str(normalized, "%Y-%m-%d") {
        Local
            .from_local_datetime(
                &value
                    .and_hms_opt(0, 0, 0)
                    .expect("valid midnight for parsed date"),
            )
            .single()
            .ok_or_else(|| {
                ThingError::InvalidInput(format!("ambiguous local date: {normalized}"))
            })?
    } else {
        return Err(ThingError::InvalidInput(format!(
            "invalid schedule target: {normalized}. expected today, tomorrow, YYYY-MM-DD, or RFC3339"
        )));
    };

    Ok(ScheduleTarget::from_local(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_thing(id: &str, name: &str) -> Thing {
        Thing {
            id: id.to_string(),
            name: name.to_string(),
            notes: None,
            status: "open".to_string(),
            tags: vec![],
            container_name: Some("Inbox".to_string()),
            container_kind: Some("list".to_string()),
            project: None,
            area: None,
            contact: None,
            due_date: None,
            activation_date: None,
            completion_date: None,
            cancellation_date: None,
            creation_date: None,
            modification_date: None,
        }
    }

    #[test]
    fn normalize_tags_sorts_and_deduplicates() {
        assert_eq!(
            normalize_tags(vec![
                " cli ".to_string(),
                "agent".to_string(),
                "cli".to_string(),
                "".to_string(),
            ]),
            vec!["agent".to_string(), "cli".to_string()]
        );
    }

    #[test]
    fn resolve_selector_prefers_id_then_name() {
        let things = vec![sample_thing("abc123", "Inbox Task")];
        assert_eq!(resolve_selector(&things, "abc123").unwrap().id, "abc123");
        assert_eq!(
            resolve_selector(&things, "Inbox Task").unwrap().name,
            "Inbox Task"
        );
    }

    #[test]
    fn resolve_selector_conflicts_on_duplicate_name() {
        let things = vec![
            sample_thing("abc123", "Inbox Task"),
            sample_thing("def456", "Inbox Task"),
        ];
        let error = resolve_selector(&things, "Inbox Task").unwrap_err();
        assert_eq!(error.exit_code(), 3);
    }

    #[test]
    fn parse_schedule_target_accepts_shortcuts() {
        let today = parse_schedule_target("today").unwrap();
        let tomorrow = parse_schedule_target("tomorrow").unwrap();
        assert!(today.year >= 2025);
        assert!(tomorrow.day >= today.day || tomorrow.month >= today.month);
    }
}
