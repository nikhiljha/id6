use tokio_postgres::{Client, Error};

pub(crate) async fn run_migrations(pg: &Client) {
    pg.execute("
        CREATE TABLE IF NOT EXISTS verifications (
        id  SERIAL,
        uuid    TEXT,
        name    TEXT,
        guild   TEXT,
        token   TEXT,
        completed   BOOLEAN)", &[])
        .await
        .expect("Unable to run database migrations.");
}

pub(crate) async fn invalidate_verification(pg: &Client, token: &str) -> Result<u64, Error> {
    pg.execute(
        "UPDATE verifications SET completed = TRUE WHERE token = $1",
        &[&token],
    ).await
}