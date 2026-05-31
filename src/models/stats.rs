use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DashboardStats {
    pub total_clients: i64,
    pub active_properties: i64,
    pub pending_deals: i64,
    pub appointments_this_week: i64,
}

#[post("/api/stats")]
#[tracing::instrument]
pub async fn get_dashboard_stats() -> Result<DashboardStats, ServerFnError> {
    tracing::info!("fetching dashboard stats");
    use sqlx::Row;
    let pool = crate::db::pool().await;

    let clients_row = sqlx::query("SELECT COUNT(*) as count FROM clients WHERE status = 'active'")
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let total_clients: i64 = clients_row.get("count");

    let props_row =
        sqlx::query("SELECT COUNT(*) as count FROM properties WHERE status = 'available'")
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    let active_properties: i64 = props_row.get("count");

    let pending_row =
        sqlx::query("SELECT COUNT(*) as count FROM properties WHERE status = 'pending'")
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    let pending_deals: i64 = pending_row.get("count");

    let appt_row = sqlx::query(
        "SELECT COUNT(*) as count FROM appointments WHERE scheduled_at >= date_trunc('week', NOW()) AND scheduled_at < date_trunc('week', NOW()) + interval '7 days'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    let appointments_this_week: i64 = appt_row.get("count");

    Ok(DashboardStats {
        total_clients,
        active_properties,
        pending_deals,
        appointments_this_week,
    })
}
