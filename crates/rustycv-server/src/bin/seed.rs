//! Loads `fixtures/adham.json` into the database.
//!
//! Used to get a realistic CV in front of the editor without retyping it, and
//! as the starting point for the FlowCV comparison.
//!
//! A CV belongs to an account, so this needs one to seed into. It signs in as
//! `SEED_EMAIL`/`SEED_PASSWORD` and creates that account if it is not there —
//! which is also how you get a development login without going through the
//! signup form.

use rustycv_core::CvDocument;
use rustycv_server::auth::{self, Credentials};

const FIXTURE: &str = include_str!("../../../../fixtures/adham.json");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://rustycv.db".to_string());
    let pool = rustycv_server::db::connect(&database_url).await?;

    let credentials = Credentials {
        email: std::env::var("SEED_EMAIL").unwrap_or_else(|_| "dev@rustycv.local".to_string()),
        password: std::env::var("SEED_PASSWORD").unwrap_or_else(|_| "rustycv-dev".to_string()),
    };

    // Sign in first so that re-seeding an existing database does not depend on
    // the account still being absent.
    let user = match auth::login(&pool, &credentials).await {
        Ok(user) => user,
        Err(_) => {
            let user = auth::signup(&pool, &credentials).await.map_err(|e| {
                anyhow::anyhow!(
                    "could not sign in as {} or create it: {e}",
                    credentials.email
                )
            })?;
            println!("created the account {}", user.email);
            user
        }
    };

    let document: CvDocument = serde_json::from_str(FIXTURE)?;
    let title = format!("{}'s CV", document.basics.full_name);

    // Re-seeding replaces the previous seed rather than piling up copies.
    let existing = rustycv_server::db::list(&pool, &user.id).await?;
    if let Some(previous) = existing.iter().find(|c| c.title == title) {
        rustycv_server::db::update(&pool, &user.id, &previous.id, Some(&title), &document).await?;
        println!("updated {} ({})", title, previous.id);
    } else {
        let cv = rustycv_server::db::create(&pool, &user.id, &title, &document).await?;
        println!("created {} ({})", title, cv.id);
    }

    println!("sign in as {}", credentials.email);
    Ok(())
}
