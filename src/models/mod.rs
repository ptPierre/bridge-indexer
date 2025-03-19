pub mod bridge;

pub struct AppState {
    pub db: sqlx::PgPool,
}