use tokio_postgres::Client;

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