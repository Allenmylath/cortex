use tokio::sync::OnceCell;

static POOL: OnceCell<sqlx::PgPool> = OnceCell::const_new();

pub async fn pool() -> &'static sqlx::PgPool {
    POOL.get_or_init(|| async {
        dotenvy::dotenv().ok();
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = sqlx::PgPool::connect(&url)
            .await
            .expect("Failed to connect to Neon Postgres");
        init_tables(&pool).await;
        pool
    })
    .await
}

async fn init_tables(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS clients (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            phone TEXT,
            budget_min BIGINT,
            budget_max BIGINT,
            preferred_areas TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            notes TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await
    .expect("Failed to create clients table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS properties (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            address TEXT NOT NULL,
            city TEXT NOT NULL,
            price BIGINT NOT NULL,
            bedrooms INTEGER,
            bathrooms INTEGER,
            area_sqft INTEGER,
            property_type TEXT NOT NULL DEFAULT 'house',
            status TEXT NOT NULL DEFAULT 'available',
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await
    .expect("Failed to create properties table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS appointments (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            client_id UUID,
            property_id UUID,
            title TEXT NOT NULL,
            scheduled_at TIMESTAMPTZ NOT NULL,
            notes TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await
    .expect("Failed to create appointments table");
}
