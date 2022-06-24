pub mod guild_ping;

use std::fs;
use std::fs::File;
use std::path::Path;

use anyhow::Result;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr, ExecResult, Schema, StatementBuilder};

pub const DB_FILE: &str = ".data/sqlite.db";

pub async fn load_database() -> Result<DatabaseConnection> {
    let path = Path::new(DB_FILE);
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap())?;
        File::create(path)?;
    }

    let database = Database::connect(format!("sqlite:{}", DB_FILE)).await?;
    execute_query(
        &database,
        Schema::new(database.get_database_backend())
            .create_table_from_entity(guild_ping::Entity)
            .if_not_exists(),
    )
    .await?;

    Ok(database)
}

pub async fn execute_query<C, S>(database: &C, query: &S) -> Result<ExecResult, DbErr>
where
    C: ConnectionTrait,
    S: StatementBuilder,
{
    let backend = database.get_database_backend();
    database.execute(backend.build(query)).await
}
