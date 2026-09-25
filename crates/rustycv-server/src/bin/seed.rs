//! Loads `fixtures/adham.json` into the database.
//!
//! Used to get a realistic CV in front of the editor without retyping it, and
//! as the starting point for the FlowCV comparison.

use rustycv_core::CvDocument;

const FIXTURE: &str = include_str!("../../../../fixtures/adham.json");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://rustycv.db".to_string());
    let pool = rustycv_server::db::connect(&database_url).await?;

    let document: CvDocument = serde_json::from_str(FIXTURE)?;
    let title = format!("{}'s CV", document.basics.full_name);

    // Re-seeding replaces the previous seed rather than piling up copies.
    let existing = rustycv_server::db::list(&pool).await?;
    if let Some(previous) = existing.iter().find(|c| c.title == title) {
        rustycv_server::db::update(&pool, &previous.id, Some(&title), &document).await?;
        println!("updated {} ({})", title, previous.id);
    } else {
        let cv = rustycv_server::db::create(&pool, &title, &document).await?;
        println!("created {} ({})", title, cv.id);
    }

    Ok(())
}
