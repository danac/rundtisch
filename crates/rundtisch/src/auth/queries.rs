use sea_query::{Asterisk, Query, SelectStatement};
use crate::auth::models::UserTable;

pub fn user_list_query() -> SelectStatement {
    Query::select()
        .column(Asterisk)
        .from(UserTable::Table)
        .to_owned()
}