// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use rusqlite::{Connection, Result as SqliteResult};

pub fn get_db_path() -> std::path::PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home_dir.join(".atci/video_info.db")
}

fn init_database(conn: &Connection) -> SqliteResult<()> {
    const SCHEMA_VERSION: &str = "20250909-3";
    
    // Create schema_version table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version TEXT PRIMARY KEY
        )",
        [],
    )?;
    
    // Check current schema version
    let current_version: Option<String> = conn.query_row(
        "SELECT version FROM schema_version LIMIT 1",
        [],
        |row| row.get(0)
    ).ok();
    
    // If version doesn't match, drop and recreate all tables
    if current_version.as_deref() != Some(SCHEMA_VERSION) {
        // Drop existing tables
        conn.execute("DROP TABLE IF EXISTS video_info", [])?;
        conn.execute("DROP TABLE IF EXISTS queue", [])?;
        conn.execute("DROP TABLE IF EXISTS currently_processing", [])?;
        conn.execute("DROP TABLE IF EXISTS schema_version", [])?;
        
        // Recreate schema_version table
        conn.execute(
            "CREATE TABLE schema_version (
                version TEXT PRIMARY KEY
            )",
            [],
        )?;
        
        // Insert current schema version
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [SCHEMA_VERSION],
        )?;
        
        // Create video_info table
        conn.execute(
            "CREATE TABLE video_info (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                base_name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                line_count INTEGER NOT NULL,
                full_path TEXT NOT NULL UNIQUE,
                transcript BOOLEAN NOT NULL,
                last_generated TEXT,
                length TEXT,
                model TEXT
            )",
            [],
        )?;
        
        // Create queue table
        conn.execute(
            "CREATE TABLE queue (
                position INTEGER PRIMARY KEY,
                path TEXT NOT NULL,
                model TEXT,
                subtitle_stream_index INTEGER
            )",
            [],
        )?;
        
        // Create currently_processing table
        conn.execute(
            "CREATE TABLE currently_processing (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                starting_time TEXT,
                path TEXT NOT NULL,
                model TEXT,
                subtitle_stream_index INTEGER
            )",
            [],
        )?;
    }
    
    Ok(())
}

pub fn get_connection() -> SqliteResult<Connection> {
    let db_path = get_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(db_path)?;
    init_database(&conn)?;
    Ok(conn)
}