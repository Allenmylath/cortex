use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Property {
    pub id: String,
    pub address: String,
    pub city: String,
    pub price: i64,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub area_sqft: Option<i32>,
    pub property_type: String,
    pub status: String,
    pub description: Option<String>,
    pub created_at: String,
}

#[post("/api/properties/list")]
pub async fn get_properties() -> Result<Vec<Property>, ServerFnError> {
    use sqlx::Row;
    let pool = crate::db::pool().await;

    let rows = sqlx::query(
        r#"SELECT id::text as id, address, city, price, bedrooms, bathrooms,
           area_sqft, property_type, status, description,
           to_char(created_at AT TIME ZONE 'UTC', 'Mon DD, YYYY') as created_at
           FROM properties ORDER BY created_at DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let props = rows
        .iter()
        .map(|r| Property {
            id: r.get("id"),
            address: r.get("address"),
            city: r.get("city"),
            price: r.get("price"),
            bedrooms: r.get("bedrooms"),
            bathrooms: r.get("bathrooms"),
            area_sqft: r.get("area_sqft"),
            property_type: r.get("property_type"),
            status: r.get("status"),
            description: r.get("description"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(props)
}

#[post("/api/properties/create")]
pub async fn create_property(
    address: String,
    city: String,
    price: i64,
    bedrooms: Option<i32>,
    bathrooms: Option<i32>,
    area_sqft: Option<i32>,
    property_type: String,
    description: String,
) -> Result<(), ServerFnError> {
    let pool = crate::db::pool().await;

    let desc_val = if description.trim().is_empty() {
        None
    } else {
        Some(description)
    };

    sqlx::query(
        "INSERT INTO properties (address, city, price, bedrooms, bathrooms, area_sqft, property_type, description) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(address)
    .bind(city)
    .bind(price)
    .bind(bedrooms)
    .bind(bathrooms)
    .bind(area_sqft)
    .bind(property_type)
    .bind(desc_val)
    .execute(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[post("/api/properties/delete")]
pub async fn delete_property(id: String) -> Result<(), ServerFnError> {
    let pool = crate::db::pool().await;

    sqlx::query("DELETE FROM properties WHERE id = $1::uuid")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
