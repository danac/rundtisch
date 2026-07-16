use sea_query::{Table, ColumnDef, TableStatement};
use crate::traits::db::{Dialect, Migration, Statement};
use super::models::*;

struct AuthMigration;

impl Migration for AuthMigration {
    fn name(&self) -> &str { "001_create_users" }

    fn up(&self) -> Vec<Statement> {
        vec![
            Statement::CreateTable(
                Table::create()
                    .table(UserTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserTable::Id).integer().not_null().auto_increment().primary_key())
                    
                    .to_owned()
            )
        ]
    }

    fn down(&self) -> Vec<Statement> {
        vec![
            Statement::DropTable(
                Table::drop()
                    .table(UserTable::Table)
                    .to_owned()
            )
        ]
    }
}