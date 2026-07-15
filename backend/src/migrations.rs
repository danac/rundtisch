use sea_query::{TableCreateStatement, TableAlterStatement, TableDropStatement};

pub enum TableStatement {
    Create(TableCreateStatement),
    Alter(TableAlterStatement),
    Drop(TableDropStatement),
}

pub trait Migration {
    fn name(&self) -> &str;
    fn up(&self)   -> Vec<TableStatement>;
    fn down(&self) -> Vec<TableStatement>;
}

pub fn all_migrations() -> Vec<Box<dyn Migration>> {
    vec![]
}