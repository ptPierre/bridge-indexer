pub mod transfers;

pub struct AppState {
    pub db: sqlx::PgPool,
}