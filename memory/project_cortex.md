---
name: project-cortex
description: RealtyPro — Dioxus 0.7 fullstack realtor helper app with Neon Postgres
metadata:
  type: project
---

Full rewrite of the cortex Dioxus 0.7 app into a realtor dashboard called RealtyPro.

**Why:** User wanted a real app (not the template demo) with DB-backed client/property management.

**How to apply:** When touching this project, respect the module boundaries and Dioxus 0.7 patterns already established.

## Stack
- Dioxus 0.7.1 fullstack (router + fullstack features)
- Neon Postgres via sqlx 0.8 (server-only dep via `cfg(not(target_arch = "wasm32"))`)
- Tailwind CSS v4 (`@import "tailwindcss"`)
- Server functions use `#[post("/api/...")]` macro (NOT `#[server]`)

## Structure
```
src/
  main.rs          — Route enum (Dashboard, Clients, Properties, Matches, Settings)
  db.rs            — lazy PgPool via tokio::sync::OnceCell; auto-inits tables on first call
  models/          — client.rs, property.rs, stats.rs (server fns + shared structs)
  components/      — layout.rs (MainLayout), sidebar.rs, stat_card.rs
  views/           — dashboard.rs, clients.rs, properties.rs, matches.rs, settings.rs
```

## DB Tables (auto-created)
- `clients` — id, name, email, phone, budget_min/max, preferred_areas, status, notes, created_at
- `properties` — id, address, city, price, bedrooms, bathrooms, area_sqft, property_type, status, description, created_at
- `appointments` — id, client_id, property_id, title, scheduled_at, notes, created_at

## Env vars
- `DATABASE_URL` — Neon Postgres connection string (see .env.example)

## Key patterns
- sqlx imports are inside `#[post]` server fn bodies (stripped for wasm by proc macro)
- `use_resource(|| async { server_fn().await })` for data; `resource.restart()` to refetch
- Slide-over modals for add-client / add-property forms
- Matches page auto-matches clients to available properties within budget range
