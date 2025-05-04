use std::{env, process::Command, time::SystemTime};

use log::info;
use sqlite::{Connection, State};

fn packet_db_loc() -> String {
    env::var("PACKETS_DB").unwrap_or("packets.db".into())
}

fn template_db_loc() -> String {
    env::var("TEMPLATE_DB").expect("TEMPLATE_DB not set!")
}

fn open_db() -> Connection {
    // Open/create a SQLite database connection
    match Connection::open(packet_db_loc()) {
        Ok(conn) => {
            info!("Opened database at {}", packet_db_loc());
            conn
        }
        Err(e) => {
            info!("Failed to open database: {}. Creating a new one.", e);
            create_db();
            Connection::open(packet_db_loc()).expect("Failed to open newly created database")
        }
    }
}

pub fn current_iteration_num() -> i64 {
    let connection = open_db();
    match connection.prepare("select (id) from iteration ORDER BY id DESC LIMIT 1;") {
        Ok(mut stm) => {
            // Execute the query and get the first row
            match stm.next() {
                Ok(State::Row) => {
                    // Read the value from the first column (id)
                    let id: i64 = stm.read::<i64, _>(0).unwrap_or(0);
                    info!("Most recent iteration ID: {}", id);
                    id // Return the most recent iteration ID incremented
                }
                Ok(State::Done) => {
                    info!("No past runs found, returning 0");
                    0 // Return 0 if no rows are found
                }
                Err(e) => {
                    info!("Error reading rows: {}", e);
                    0 // Return 0 on error
                }
            }
        }

        _ => {
            info!("Couldn't read rows, probably no past runs...");
            0
        }
    }
}

pub fn db_init() {
    // See if DB file exists
    let db_path = packet_db_loc();
    if std::path::Path::new(&db_path).exists() {
        info!("Database file exists at {}", db_path);
    } else {
        info!("Database file does not exist at {}, creating it.", db_path);
        create_db();
    }

    start_iteration();
}

fn start_iteration() {
    // Open the database connection
    let connection = open_db();

    // Prepare query
    let this_iter = current_iteration_num() + 1;
    let system_start_time = now_millis(); // Get the current time in milliseconds
    let query = format!(
        "INSERT INTO iteration (id, local_time_boot, boot_num) VALUES ({}, {}, 0);",
        this_iter, system_start_time
    );

    // Execute the query
    if let Err(e) = connection.execute(&query) {
        info!("Failed to set new iteration: {:?}", e);
    }
}

// Returns the current time in milliseconds since the Unix epoch
fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards!")
        .as_millis()
}

fn create_db() {
    Command::new("cp")
        .arg(template_db_loc())
        .arg(packet_db_loc())
        .output()
        .unwrap();
    info!("Created new database");
}
