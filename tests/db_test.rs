use ava_device_logger::database::Database;
use std::error::Error;

#[tokio::test]
async fn test_get_devices_from_real_db() -> Result<(), Box<dyn Error>> {
    // Allow overriding the DB path via TEST_DB_PATH env var for flexibility in CI
    let db_path = std::env::var("TEST_DB_PATH").unwrap_or_else(|_| "data.db".to_string());

    // Open the real database (read-only operations only)
    let db = Database::new(&db_path).await?;

    // Fetch all devices and print summary
    let devices = db.get_devices().await?;

    println!("Using DB: {}", db_path);
    println!("Found {} devices", devices.len());
    for d in &devices {
        println!("- id={} name={} model_id={:?} enabled={}", d.id, d.name, d.model_id, d.enabled);
    }

    // Test passes if function returns successfully. Do not mutate the real DB in this test.
    Ok(())
}
