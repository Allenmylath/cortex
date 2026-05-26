use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Client {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub budget_min: Option<i64>,
    pub budget_max: Option<i64>,
    pub preferred_areas: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
}

#[post("/api/clients/list")]
pub async fn get_clients() -> Result<Vec<Client>, ServerFnError> {
    use sqlx::Row;
    let pool = crate::db::pool().await;

    let rows = sqlx::query(
        r#"SELECT id::text as id, name, email, phone,
           budget_min, budget_max, preferred_areas, status, notes,
           to_char(created_at AT TIME ZONE 'UTC', 'Mon DD, YYYY') as created_at
           FROM clients ORDER BY created_at DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let clients = rows
        .iter()
        .map(|r| Client {
            id: r.get("id"),
            name: r.get("name"),
            email: r.get("email"),
            phone: r.get("phone"),
            budget_min: r.get("budget_min"),
            budget_max: r.get("budget_max"),
            preferred_areas: r.get("preferred_areas"),
            status: r.get("status"),
            notes: r.get("notes"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(clients)
}

#[post("/api/clients/create")]
pub async fn create_client(
    name: String,
    email: String,
    phone: String,
    budget_min: Option<i64>,
    budget_max: Option<i64>,
    preferred_areas: String,
    notes: String,
) -> Result<(), ServerFnError> {
    let pool = crate::db::pool().await;

    let phone_val = if phone.trim().is_empty() {
        None
    } else {
        Some(phone)
    };
    let areas_val = if preferred_areas.trim().is_empty() {
        None
    } else {
        Some(preferred_areas)
    };
    let notes_val = if notes.trim().is_empty() {
        None
    } else {
        Some(notes)
    };

    sqlx::query(
        "INSERT INTO clients (name, email, phone, budget_min, budget_max, preferred_areas, notes) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(name)
    .bind(email)
    .bind(phone_val)
    .bind(budget_min)
    .bind(budget_max)
    .bind(areas_val)
    .bind(notes_val)
    .execute(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[post("/api/clients/delete")]
pub async fn delete_client(id: String) -> Result<(), ServerFnError> {
    let pool = crate::db::pool().await;

    sqlx::query("DELETE FROM clients WHERE id = $1::uuid")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
