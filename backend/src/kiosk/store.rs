//! Kiosk tables: schema, seed menu, menu listing, order creation.
//!
//! The DDL is what Rails 8.0.2 created from `db/schema.rb` (read back from
//! `sqlite_master`), so the data file has the same shape as the Rails one.

use super::encode;
use crate::db::{DbError, Pool, Row, Value};

/// Tables and indexes from `db/schema.rb` (version 2025_07_18_020859).
const SCHEMA: &[&str] = &[
    r#"CREATE TABLE IF NOT EXISTS "menu_items" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "name" varchar, "category" varchar, "created_at" datetime(6) NOT NULL, "updated_at" datetime(6) NOT NULL, "price" decimal)"#,
    r#"CREATE TABLE IF NOT EXISTS "orders" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "total" decimal, "status" varchar, "created_at" datetime(6) NOT NULL, "updated_at" datetime(6) NOT NULL, "items" text)"#,
    r#"CREATE TABLE IF NOT EXISTS "order_items" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "order_id" integer NOT NULL, "menu_item_id" integer NOT NULL, "quantity" integer, "created_at" datetime(6) NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_5ab12df479" FOREIGN KEY ("menu_item_id") REFERENCES "menu_items" ("id"), CONSTRAINT "fk_rails_e3cb28f071" FOREIGN KEY ("order_id") REFERENCES "orders" ("id"))"#,
    r#"CREATE INDEX IF NOT EXISTS "index_order_items_on_menu_item_id" ON "order_items" ("menu_item_id")"#,
    r#"CREATE INDEX IF NOT EXISTS "index_order_items_on_order_id" ON "order_items" ("order_id")"#,
    r#"CREATE TABLE IF NOT EXISTS "schema_migrations" ("version" varchar NOT NULL PRIMARY KEY)"#,
];

/// The five Rails migrations, recorded so the file reads as fully migrated.
const MIGRATIONS: &[&str] = &[
    "20250718013132",
    "20250718013211",
    "20250718013247",
    "20250718013350",
    "20250718020859",
];

/// `db/seeds.rb`, in file order (ids 1..19, category NULL).
pub const SEED_MENU: &[(&str, &str)] = &[
    ("Hotdog & Soda", "1.50"),
    ("Slice Pepperoni", "1.99"),
    ("Slice Cheese", "1.99"),
    ("Rotisserie Chicken Caesar Salad", "6.99"),
    ("Whole Pepperoni", "9.95"),
    ("Whole Cheese", "9.95"),
    ("1/2 Pepperoni 1/2 Cheese", "9.95"),
    ("Chicken Bake", "3.99"),
    ("20 oz Soda with Refill", "0.79"),
    ("Double Chocolate Chunk Cookie", "2.49"),
    ("Hot Turkey Provolone Sandwich", "6.99"),
    ("Strawberry Banana Smoothie", "2.99"),
    ("Cold Brew Mocha Freeze", "2.99"),
    ("Cold Brew Latte Freeze", "2.99"),
    ("Vanilla Ice Cream Cup", "1.99"),
    ("Chocolate Ice Cream Cup", "1.99"),
    ("Chocolate Vanilla Twist Ice Cream", "1.99"),
    ("Chocolate Sundae", "2.49"),
    ("Strawberry Sundae", "2.49"),
];

/// Create the kiosk schema and seed the menu, like `bin/rails db:prepare`.
///
/// Seeds run only when `menu_items` did not exist yet, which is when Rails'
/// `db:prepare` runs `db/seeds.rb` (on database creation). A restart on a
/// persisted volume keeps the existing menu and orders.
///
/// # Errors
/// Any SQLite failure; the caller refuses to start.
pub fn prepare(pool: &Pool) -> Result<bool, DbError> {
    pool.transaction(|db| {
        let existed = !db
            .query(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'menu_items'",
                &[],
            )?
            .is_empty();
        for statement in SCHEMA {
            db.exec(statement)?;
        }
        for version in MIGRATIONS {
            db.run(
                "INSERT OR IGNORE INTO schema_migrations (version) VALUES (?)",
                &[Value::Text((*version).to_string())],
            )?;
        }
        if existed {
            return Ok(false);
        }
        for (name, price) in SEED_MENU {
            let now = Value::Text(encode::db_timestamp(encode::now_micros()));
            db.run(
                "INSERT INTO menu_items (name, created_at, updated_at, price) VALUES (?, ?, ?, ?)",
                &[
                    Value::Text((*name).to_string()),
                    now.clone(),
                    now,
                    Value::Text((*price).to_string()),
                ],
            )?;
        }
        Ok(true)
    })
}

/// `MenuItem.all.to_json`: every row in id order, columns in table order.
///
/// # Errors
/// Any SQLite failure.
pub fn menu_json(pool: &Pool) -> Result<String, DbError> {
    let rows = pool.with(|db| {
        db.query(
            "SELECT id, name, category, created_at, updated_at, price FROM menu_items ORDER BY id",
            &[],
        )
    })?;
    let mut out = String::from("[");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_menu_item(&mut out, row);
    }
    out.push(']');
    Ok(out)
}

fn push_menu_item(out: &mut String, row: &Row) {
    out.push_str("{\"id\":");
    push_int(out, row.get("id"));
    out.push_str(",\"name\":");
    push_text(out, row.get("name"));
    out.push_str(",\"category\":");
    push_text(out, row.get("category"));
    out.push_str(",\"created_at\":");
    push_time(out, row.get("created_at"));
    out.push_str(",\"updated_at\":");
    push_time(out, row.get("updated_at"));
    out.push_str(",\"price\":");
    push_decimal(out, row.get("price"));
    out.push('}');
}

fn push_int(out: &mut String, v: Option<&Value>) {
    match v {
        Some(Value::Int(n)) => out.push_str(&n.to_string()),
        _ => out.push_str("null"),
    }
}

fn push_text(out: &mut String, v: Option<&Value>) {
    match v {
        Some(Value::Text(s)) => encode::push_str(out, s),
        Some(Value::Int(n)) => encode::push_str(out, &n.to_string()),
        Some(Value::Real(x)) => encode::push_str(out, &x.to_string()),
        _ => out.push_str("null"),
    }
}

fn push_time(out: &mut String, v: Option<&Value>) {
    match v {
        Some(Value::Text(s)) => encode::push_str(out, &encode::json_timestamp(s)),
        _ => out.push_str("null"),
    }
}

/// A `decimal` column as Rails renders a `BigDecimal`: a JSON string.
fn push_decimal(out: &mut String, v: Option<&Value>) {
    let x = match v {
        Some(Value::Real(x)) => *x,
        Some(Value::Int(n)) => *n as f64,
        Some(Value::Text(s)) => s.trim().parse::<f64>().unwrap_or(0.0),
        _ => {
            out.push_str("null");
            return;
        }
    };
    encode::push_str(out, &encode::decimal_string(x));
}

/// `Order.create!(total:, items:)`. Returns the new order id.
///
/// `total` is decimal text (or `None` for NULL) and `items` the serialized
/// JSON text (or `None`), both prepared by the params layer.
///
/// # Errors
/// Any SQLite failure; the route answers 422 with its message.
pub fn create_order(
    pool: &Pool,
    total: Option<String>,
    items: Option<String>,
) -> Result<i64, DbError> {
    let now = Value::Text(encode::db_timestamp(encode::now_micros()));
    let opt = |v: Option<String>| v.map_or(Value::Null, Value::Text);
    let changes = pool.with(|db| {
        db.run(
            "INSERT INTO orders (total, status, created_at, updated_at, items) VALUES (?, NULL, ?, ?, ?)",
            &[opt(total), now.clone(), now, opt(items)],
        )
    })?;
    Ok(changes.last_insert_rowid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_pool() -> Pool {
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("kiosk-store-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("Kiosk.db");
        let _ = std::fs::remove_file(&path);
        Pool::open(&path.to_string_lossy(), 2).expect("pool")
    }

    fn scalar(pool: &Pool, sql: &str) -> Value {
        let rows = pool.with(|db| db.query(sql, &[])).expect("query");
        rows.first()
            .and_then(|r| r.get("v").cloned())
            .unwrap_or(Value::Null)
    }

    #[test]
    fn prepare_seeds_nineteen_items_once() {
        let pool = temp_pool();
        assert!(prepare(&pool).expect("first"));
        assert!(!prepare(&pool).expect("second"));
        assert_eq!(
            scalar(&pool, "SELECT count(*) AS v FROM menu_items"),
            Value::Int(19)
        );
    }

    #[test]
    fn seeded_prices_store_as_real_like_rails() {
        let pool = temp_pool();
        prepare(&pool).expect("prepare");
        assert_eq!(
            scalar(
                &pool,
                "SELECT typeof(price) AS v FROM menu_items WHERE id = 1"
            ),
            Value::Text("real".into())
        );
    }

    #[test]
    fn records_rails_migration_versions() {
        let pool = temp_pool();
        prepare(&pool).expect("prepare");
        assert_eq!(
            scalar(&pool, "SELECT count(*) AS v FROM schema_migrations"),
            Value::Int(5)
        );
    }

    #[test]
    fn menu_json_matches_rails_shape() {
        let pool = temp_pool();
        prepare(&pool).expect("prepare");
        pool.with(|db| {
            db.run(
                "UPDATE menu_items SET created_at = ?, updated_at = ? WHERE id = 1",
                &[
                    Value::Text("2026-10-28 01:10:18.559279".into()),
                    Value::Text("2026-10-28 01:10:18.559279".into()),
                ],
            )
        })
        .expect("update");
        let json = menu_json(&pool).expect("json");
        // First element of the live https://kiosk.bixbyapps.com/menu_items response.
        let first = r#"[{"id":1,"name":"Hotdog \u0026 Soda","category":null,"created_at":"2026-10-28T01:10:18.559Z","updated_at":"2026-10-28T01:10:18.559Z","price":"1.5"},"#;
        assert!(json.starts_with(first), "{json}");
    }

    #[test]
    fn menu_json_lists_seed_order() {
        let pool = temp_pool();
        prepare(&pool).expect("prepare");
        let json = menu_json(&pool).expect("json");
        assert!(json.contains(r#"{"id":19,"name":"Strawberry Sundae","category":null,"#));
        assert!(json.contains(r#""price":"0.79"}"#));
    }

    #[test]
    fn create_order_applies_numeric_affinity() {
        let pool = temp_pool();
        prepare(&pool).expect("prepare");
        let id = create_order(&pool, Some("9.99".into()), Some("[1]".into())).expect("create");
        assert_eq!(id, 1);
        let zero = create_order(&pool, Some("0".into()), None).expect("create");
        assert_eq!(
            scalar(
                &pool,
                &format!("SELECT typeof(total) AS v FROM orders WHERE id = {zero}")
            ),
            Value::Text("integer".into())
        );
        let rounded =
            create_order(&pool, Some("9.999999999999999999".into()), None).expect("create");
        assert_eq!(
            scalar(
                &pool,
                &format!("SELECT total AS v FROM orders WHERE id = {rounded}")
            ),
            Value::Int(10)
        );
    }

    #[test]
    fn create_order_null_total() {
        let pool = temp_pool();
        prepare(&pool).expect("prepare");
        let id = create_order(&pool, None, None).expect("create");
        assert_eq!(
            scalar(
                &pool,
                &format!("SELECT total AS v FROM orders WHERE id = {id}")
            ),
            Value::Null
        );
    }
}
