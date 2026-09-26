use sea_orm::DatabaseConnection;
use sea_orm::DbErr;

/// Open the database named by `DATABASE_URL`.
///
/// `sqlite://`, `mysql://`, and `postgres://` are all valid. This is the only
/// place the demo chooses a backend.
pub async fn connect() -> Result<DatabaseConnection, DbErr> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    sea_orm::Database::connect(&url).await
}
