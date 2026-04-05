use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "things-cli",
    version,
    about = "things-cli: public CLI for agents working with the real Things 3 app"
)]
pub struct Cli {
    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List available Things lists.
    Lists,
    /// List Things to-dos.
    List(ListArgs),
    /// Fetch a to-do by id or exact name.
    #[command(alias = "show")]
    Get(SelectorArgs),
    /// Create a new to-do.
    Create(CreateArgs),
    /// Update a to-do by id or exact name.
    Update(UpdateArgs),
    /// Mark a to-do as completed.
    Complete(SelectorArgs),
    /// Move a to-do to another Things list.
    Move(MoveArgs),
    /// Schedule a to-do for a date or time.
    Schedule(ScheduleArgs),
    /// Delete a to-do by id or exact name.
    Delete(SelectorArgs),
    /// Reveal a to-do in Things.
    Open(SelectorArgs),
}

#[derive(Debug, Args)]
pub struct SelectorArgs {
    pub selector: String,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Case-insensitive substring match across key fields.
    #[arg(long)]
    pub query: Option<String>,

    #[arg(long)]
    pub status: Option<String>,

    #[arg(long = "tag")]
    pub tags: Vec<String>,

    #[arg(long = "list")]
    pub list_name: Option<String>,

    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    pub name: String,

    #[arg(long, conflicts_with = "notes_file")]
    pub notes: Option<String>,

    #[arg(long, value_name = "PATH|-", conflicts_with = "notes")]
    pub notes_file: Option<String>,

    #[arg(long = "list", default_value = "Inbox")]
    pub list_name: String,

    #[arg(long = "tag")]
    pub tags: Vec<String>,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    pub selector: String,

    #[arg(long)]
    pub name: Option<String>,

    #[arg(long, conflicts_with = "notes_file")]
    pub notes: Option<String>,

    #[arg(long, value_name = "PATH|-", conflicts_with = "notes")]
    pub notes_file: Option<String>,

    #[arg(long, conflicts_with_all = ["notes", "notes_file"])]
    pub clear_notes: bool,

    #[arg(long)]
    pub status: Option<String>,

    #[arg(long = "tag")]
    pub tags: Vec<String>,

    #[arg(long)]
    pub clear_tags: bool,
}

#[derive(Debug, Args)]
pub struct MoveArgs {
    pub selector: String,

    #[arg(long = "to")]
    pub list_name: String,
}

#[derive(Debug, Args)]
pub struct ScheduleArgs {
    pub selector: String,

    /// Accepts today, tomorrow, YYYY-MM-DD, or RFC3339.
    #[arg(long = "for")]
    pub when: String,
}
