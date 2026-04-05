use serde::Serialize;

use crate::model::{Thing, ThingList};

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Output {
    Lists { lists: Vec<ThingList> },
    Thing { thing: Thing },
    Things { things: Vec<Thing> },
    Deleted { deleted: bool, thing: Thing },
    Opened { opened: bool, thing: Thing },
}
