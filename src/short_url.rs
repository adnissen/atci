// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::db;
use rand::Rng;
use rusqlite::Result as SqliteResult;

/// Generates a random 5-character alphanumeric ID
#[allow(dead_code)]
fn generate_id() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();

    (0..5)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Gets an existing short URL or creates a new one
/// If the generated ID already exists, it will be overwritten with the new URL
#[allow(dead_code)]
pub fn get_or_create(url: &str) -> SqliteResult<String> {
    let conn = db::get_connection()?;

    // First, check if this URL already has an ID
    let existing_id: Option<String> = conn
        .query_row("SELECT id FROM short_urls WHERE url = ?1", [url], |row| {
            row.get(0)
        })
        .ok();

    if let Some(id) = existing_id {
        return Ok(id);
    }

    // Generate a new ID
    let id = generate_id();

    // Insert or replace (if ID exists, it will be overwritten)
    conn.execute(
        "INSERT OR REPLACE INTO short_urls (id, url) VALUES (?1, ?2)",
        [&id, url],
    )?;

    Ok(id)
}

/// Gets the URL associated with a short ID
pub fn get_url(id: &str) -> SqliteResult<Option<String>> {
    let conn = db::get_connection()?;

    let url: Option<String> = conn
        .query_row("SELECT url FROM short_urls WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .ok();

    Ok(url)
}
