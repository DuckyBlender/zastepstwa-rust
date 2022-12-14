#[macro_use]
extern crate rocket;

use std::path::{Path, PathBuf};

use chrono::Datelike;
use log::{error, info, warn};
use rocket::fs::NamedFile;
use rocket::tokio::io::AsyncWriteExt;
use serde_json::json;

#[get("/?<day>&<month>")]
async fn get_data(day: u8, month: u8) -> Result<NamedFile, serde_json::Value> {
    info!("Incoming request for {}.{}", day, month);
    if day > 31 || month > 12 {
        warn!("Invalid date: {}/{}", day, month);
        return Err(json!({
            "error": "Invalid date"
        }));
    }
    // Make the day have 2 digits
    let day = format!("{:02}", day);
    let current_year = chrono::Local::now().year();
    let date = format!("{}.{}.{}", day, month, current_year);

    // Check if the file is in the cache
    let filename_pdf = format!("./cached/{}.pdf", date);
    if std::path::Path::new(&filename_pdf).exists() {
        // Check if the file is younger then 10 minutes
        let metadata = rocket::tokio::fs::metadata(&filename_pdf)
            .await
            .expect("Error while getting metadata");
        let file_age = chrono::Local::now()
            - chrono::DateTime::from(
                metadata
                    .modified()
                    .expect("Error while getting file modified date"),
            );
        if file_age.num_minutes() < 10 {
            // Return the file
            info!("Returning cached data for {}", date);
            return Ok(NamedFile::open(&filename_pdf)
                .await
                .expect("Error while opening file"));
        } else {
            // Delete the file
            info!("Deleting old cached data for {}", date);
            rocket::tokio::fs::remove_file(&filename_pdf)
                .await
                .expect("Error while deleting file");
            // And continue as normal
        }
    }

    info!("Getting data for {}", date);
    let response =
        match reqwest::get(format!("https://zastepstwa.zschie.pl/pliki/{}.pdf", date)).await {
            Ok(response) => response,
            Err(err) => {
                error!("Error while getting data: {}", err);
                return Err(json!({
                    "error": "Szko??a jest offline! Spr??buj ponownie p????niej."
                }));
            }
        };

    // If the server returns a 200 status code
    if response.status() == 200 {
        // Create a new file
        let filename_pdf = format!("./cached/{}.pdf", date);

        let mut file = match rocket::tokio::fs::File::create(&filename_pdf).await {
            Ok(file) => file,
            Err(err) => {
                error!("Error #1 while creating file: {}", err);
                return Err(json!({
                    "error": "Error #1, zg??o?? ten problem do tw??rcy!"
                }));
            }
        };
        // Download the PDF
        let filebytes = match response.bytes().await {
            Ok(filebytes) => filebytes,
            Err(err) => {
                error!("Error #2 while downloading file: {}", err);
                return Err(json!({
                    "error": "Error #2, zg??o?? ten problem do tw??rcy!"
                }));
            }
        };
        // Write the PDF to the file
        match file.write_all(&filebytes).await {
            Ok(file) => file,
            Err(err) => {
                error!("Error #3 while writing file: {}", err);
                return Err(json!({
                    "error": "Error #3, zg??o?? ten problem do tw??rcy!"
                }));
            }
        };

        // Return the PDF
        Ok(NamedFile::open(&filename_pdf)
            .await
            .expect("Error while opening file"))
    } else if response.status() == 404 {
        warn!("No data for {}", date);
        // If the server returns a 404 status code
        Err(json!({
            "error": format!("Nie ma obecnie zast??pstw na dzie?? {}", date)
        }))
    } else {
        // Return an error
        let response_status = response.status().as_u16();
        error!("Server returned a {} status code", response_status);
        Err(json!({
            "error":
                format!(
                    "Serwer zwr??ci?? nieznany status {}! Spr??buj ponownie p????niej",
                    response_status
                )
        }))
    }
}

#[get("/?<when>")]
async fn auto_get_data(when: String) -> Result<NamedFile, serde_json::Value> {
    // Get current date
    let current_date = if when == "tomorrow" {
        // If it's friday or saturday return message
        match chrono::Local::now().weekday() {
            chrono::Weekday::Fri => {
                return Err(json!({"error": "Jest jutro sobota, wi??c nie ma zast??pstw!"}))
            }
            chrono::Weekday::Sat => {
                return Err(json!({"error": "Jest jutro niedziela, wi??c nie ma zast??pstw!"}))
            }
            _ => chrono::Local::now() + chrono::Duration::days(1),
        }
    } else if when == "today" {
        match chrono::Local::now().weekday() {
            chrono::Weekday::Sat => {
                return Err(json!({"error": "Jest dzi?? sobota, nie ma dzi?? ??adnych lekcji!"}))
            }
            chrono::Weekday::Sun => {
                return Err(json!({"error": "Jest dzi?? niedziela, nie ma dzi?? ??adnych lekcji!"}))
            }
            _ => chrono::Local::now(),
        }
    } else {
        error!("Invalid parameter for when: {}", when);
        return Err(json!({"error": "Niepoprawny parametr!"}));
    };
    info!(
        "Incoming request for {} ({}.{})",
        when,
        current_date.day(),
        current_date.month()
    );

    // Format the current date to the PL format
    let date = current_date.format("%d.%m.%Y").to_string();
    // Send a get request to the server

    // Check if the file is in the cache
    let filename_pdf = format!("./cached/{}.pdf", date);
    if std::path::Path::new(&filename_pdf).exists() {
        // Check if the file is younger then 10 minutes
        let metadata = rocket::tokio::fs::metadata(&filename_pdf)
            .await
            .expect("Error while getting metadata");
        let file_age = chrono::Local::now()
            - chrono::DateTime::from(
                metadata
                    .modified()
                    .expect("Error while getting file modified date"),
            );
        if file_age.num_minutes() < 10 {
            // Return the file
            info!("Returning cached data for {}", date);
            return Ok(NamedFile::open(&filename_pdf)
                .await
                .expect("Error while opening file"));
        } else {
            // Delete the file
            info!("Deleting old cached data for {}", date);
            rocket::tokio::fs::remove_file(&filename_pdf)
                .await
                .expect("Error while deleting file");
            // And continue as normal
        }
    }

    info!("Getting data for {}", date);
    let response =
        match reqwest::get(format!("https://zastepstwa.zschie.pl/pliki/{}.pdf", date)).await {
            Ok(response) => response,
            Err(err) => {
                error!("Error while getting data: {}", err);
                return Err(json!({
                    "error": "Szko??a jest offline! Spr??buj ponownie p????niej."
                }));
            }
        };

    // If the server returns a 200 status code
    if response.status() == 200 {
        // Create a new file
        let filename_pdf = format!("./cached/{}.pdf", date);

        let mut file = match rocket::tokio::fs::File::create(&filename_pdf).await {
            Ok(file) => file,
            Err(err) => {
                error!("Error #1 while creating file: {}", err);
                return Err(json!({
                    "error": "Error #1, zg??o?? ten problem do tw??rcy!"
                }));
            }
        };
        // Download the PDF
        let filebytes = match response.bytes().await {
            Ok(filebytes) => filebytes,
            Err(err) => {
                error!("Error while downloading file: {}", err);
                return Err(json!({
                    "error": "Error #2, zg??o?? ten problem do tw??rcy!"
                }));
            }
        };
        // Write the PDF to the file
        match file.write_all(&filebytes).await {
            Ok(file) => file,
            Err(err) => {
                error!("Error while writing file: {}", err);
                return Err(json!({
                    "error": "Error #3, zg??o?? ten problem do tw??rcy!"
                }));
            }
        };

        // Return the file
        match NamedFile::open(&filename_pdf).await {
            Ok(file) => Ok(file),
            Err(err) => {
                error!("Error while opening file: {}", err);
                Err(json!({
                    "error": "Error #4, zg??o?? ten problem do tw??rcy!"
                }))
            }
        }
    } else if response.status() == 404 {
        // If the server returns a 404 status code
        warn!("No data for {}", date);
        Err(json!({
            "error": format!("Nie ma obecnie zast??pstw na dzie?? {}", date)
        }))
    } else {
        // Return an error
        let response_status = response.status().as_u16();
        error!("Server returned a {} status code", response_status);
        Err(json!({
            "error":
                format!(
                    "Serwer zwr??ci?? nieznany status {}! Spr??buj ponownie p????niej",
                    response_status
                )
        }))
    }
}

// File serving (for example, localhost:9000/files/10.10.2022.pdf)
#[get("/<file>")]
async fn files(file: &str) -> NamedFile {
    NamedFile::open(format!("./cached/{}", file))
        .await
        .expect("Error while opening file")
}

// Status page
#[get("/")]
async fn status() -> &'static str {
    "Strona jest online!"
}

// 404 handler
#[catch(404)]
async fn not_found() -> &'static str {
    "Nie ma takiej strony! Je??li uwa??asz ??e to b????d, napisz do tw??rcy."
}

#[launch]
async fn launch() -> _ {
    // Check if the cached folder exists
    if !std::path::Path::new("./cached").exists() {
        // If it doesn't, create it
        rocket::tokio::fs::create_dir("./cached")
            .await
            .expect("Error while creating cached folder");
    }
    // Don't check for the log or config file, because they are in the Github repo

    // Start the server
    rocket::build()
        // Static files
        .mount("/", routes![get_data])
        .mount("/auto/", routes![auto_get_data])
        .mount("/status/", routes![status])
        .mount("/files/", routes![files])
        .register("/", catchers![not_found])
}
