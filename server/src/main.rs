use chrono::Utc;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{
    Request, Response,
    body::{Body, Bytes, Incoming},
    header::USER_AGENT,
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;

/**
 * server
 *
 * ## new_item - create a new item
```json
{
    "name": "item name",
    "barcode": "42",
    "location": "location",
    "last-seen": 1234567890 // unix timestamp
}
```

table creation:
```sql
CREATE TABLE items (
    name VARCHAR NOT NULL,
    barcode INTEGER NOT NULL UNIQUE,
    location VARCHAR NOT NULL,
    last_seen TIMESTAMP NOT NULL
);
````
 */

#[cfg(test)]
const DB_NAME: &str = "test.db";

#[cfg(not(test))]
const DB_NAME: &str = "barcode.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    name: String,
    barcode: u64,
    location: String,
    last_seen: Option<u64>,
}

impl Item {
    pub fn new(name: String, barcode: u64, location: String) -> Self {
        Self {
            name,
            barcode,
            location,
            last_seen: Some(Utc::now().timestamp() as u64),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let conn = Connection::open(DB_NAME).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO items (name, barcode, location, last_seen) VALUES (?1, ?2, ?3, ?4)",
            params![self.name, self.barcode, self.location, self.last_seen],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn load_items() -> Result<Vec<Item>, String> {
    let conn = Connection::open(DB_NAME).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT name, barcode, location, last_seen FROM items")
        .map_err(|e| e.to_string())?;
    let items = stmt
        .query_map(params![], |row| {
            Ok(Item {
                name: row.get(0)?,
                barcode: row.get(1)?,
                location: row.get(2)?,
                last_seen: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .map(|r| r.map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

pub fn load_item(barcode: u64) -> Result<Item, String> {
    let conn = Connection::open(DB_NAME).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT name, barcode, location, last_seen FROM items WHERE barcode = ?1")
        .map_err(|e| e.to_string())?;
    let item = stmt
        .query_map(params![barcode], |row| {
            Ok(Item {
                name: row.get(0)?,
                barcode: row.get(1)?,
                location: row.get(2)?,
                last_seen: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .map(|r| r.map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if item.len() == 0 {
        return Err("Item not found".to_string());
    }
    Ok(item[0].clone()) // UNIQUE constraint on barcode, so there will be only one item
}

pub fn delete_item(barcode: &str) -> Result<(), String> {
    let conn = Connection::open(DB_NAME).map_err(|e| e.to_string())?;
    let rows_affected = conn
        .execute("DELETE FROM items WHERE barcode = ?1", params![barcode])
        .map_err(|e| e.to_string())?;
    if rows_affected == 0 {
        return Err("Item not found".to_string());
    }
    Ok(())
}

pub fn modify_item(item: Item) -> Result<(), String> {
    let conn = Connection::open(DB_NAME).map_err(|e| e.to_string())?;
    let rows_affected = conn
        .execute(
            "UPDATE items SET name = ?1, location = ?2, last_seen = ?3 WHERE barcode = ?4",
            params![item.name, item.location, item.last_seen, item.barcode],
        )
        .map_err(|e| e.to_string())?;

    if rows_affected == 0 {
        return Err("Item not found".to_string());
    }

    Ok(())
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn ok() -> BoxBody<Bytes, hyper::Error> {
    full("OK")
}

/// remove all non-alphanumeric characters from a string (all fields can have this applied)
fn sanitize(s: &str) -> String {
    s.replace(
        |c: char| !c.is_ascii_alphanumeric() && !c.is_ascii_whitespace(),
        "",
    )
}

impl Item {
    /// remove all non-alphanumeric characters from all fields
    fn sanitize(&mut self) {
        self.name = sanitize(&self.name);
        self.location = sanitize(&self.location);
    }
}

// endpoint for new item (hyper)
async fn new_item(
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let max = req.body().size_hint().upper().unwrap_or(u64::MAX);
    if max > 1024 * 64 {
        let mut resp = Response::new(full("Body too big"));
        *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
        return Ok(resp);
    }

    let whole_body = req.collect().await?.to_bytes().to_vec();

    let str_body = std::str::from_utf8(&whole_body);

    let item: Result<Item, serde_json::Error> = serde_json::from_str(str_body.unwrap());

    if item.is_err() {
        let mut resp = Response::new(full("Invalid JSON"));
        *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
        return Ok(resp);
    }


    // now give it a last seen time of now
    let mut item = item.unwrap(); // unwrap is safe because we checked it above
    item.sanitize();
    item.last_seen = Some(Utc::now().timestamp() as u64);

    let res = item.save();

    if let Err(err) = res {
        let mut resp = if err.contains("UNIQUE constraint failed") {
            Response::new(full("Item already exists"))
        } else {
            Response::new(full(err.clone()))
        };
        *resp.status_mut() = if err.contains("UNIQUE constraint failed") {
            hyper::StatusCode::CONFLICT
        } else {
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        };
        return Ok(resp);
    }

    Ok(Response::new(ok()))
}

// endpoint for all items (hyper)
async fn all_items(
    _req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let items = load_items();

    if items.is_err() {
        let mut resp = Response::new(full(items.unwrap_err()));
        *resp.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
        return Ok(resp);
    }

    let items: Vec<Item> = items
        .unwrap()
        .iter_mut()
        .map(|i| {
            i.sanitize();
            i.clone()
        })
        .collect();

    let items_json = serde_json::to_string(&items);

    if items_json.is_err() {
        let mut resp = Response::new(full(items_json.unwrap_err().to_string()));
        *resp.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
        return Ok(resp);
    }

    Ok(Response::new(full(items_json.unwrap()))) // unwrap is safe because we checked it above
}

// endpoint for item (hyper)
async fn item(
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let barcode = req.uri().path().split('/').last();

    let barcode = match barcode {
        Some(barcode) => match barcode.parse::<u64>() {
            Ok(barcode) => barcode,
            Err(_) => {
                let mut resp = Response::new(full("Invalid barcode"));
                *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
                return Ok(resp);
            }
        },
        None => {
            let mut resp = Response::new(full("No barcode"));
            *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
            return Ok(resp);
        }
    };

    let item = load_item(barcode); // unwrap is safe because we checked it above

    if let Err(err) = item {
        let mut resp = if err == "Item not found" {
            Response::new(full("Item not found"))
        } else {
            Response::new(full(err.clone()))
        };
        *resp.status_mut() = if err == "Item not found" {
            hyper::StatusCode::NOT_FOUND
        } else {
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        };
        return Ok(resp);
    }

    let mut item = item.unwrap(); // unwrap is safe because we checked it above
    item.sanitize();

    let item_json = serde_json::to_string(&item);

    if item_json.is_err() {
        let mut resp = Response::new(full(item_json.unwrap_err().to_string()));
        *resp.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
        return Ok(resp);
    }

    Ok(Response::new(full(item_json.unwrap()))) // unwrap is safe because we checked it above
}

// endpoint to modify item (hyper)
// expected format:
/*
```
{
    "name": "item name",
    "barcode": "42",
    "location": "location",
}
```
*/
async fn modify_item_endpoint(
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let max = req.body().size_hint().upper().unwrap_or(u64::MAX);
    if max > 1024 * 64 {
        let mut resp = Response::new(full("Body too big"));
        *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
        return Ok(resp);
    }

    let whole_body = req.collect().await?.to_bytes().to_vec();

    let str_body = std::str::from_utf8(&whole_body);

    let item: Result<Item, serde_json::Error> = serde_json::from_str(str_body.unwrap());

    if item.is_err() {
        let mut resp = Response::new(full("Invalid JSON"));
        *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
        return Ok(resp);
    }

    let mut item = item.unwrap(); // unwrap is safe because we checked it above
    item.sanitize();
    item.last_seen = Some(Utc::now().timestamp() as u64);

    let res = modify_item(item);

    if let Err(err) = res {
        let mut resp = if err == "Item not found" {
            Response::new(full("Item not found"))
        } else {
            Response::new(full(err.clone()))
        };
        *resp.status_mut() = if err == "Item not found" {
            hyper::StatusCode::NOT_FOUND
        } else {
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        };
        return Ok(resp);
    }

    Ok(Response::new(ok()))
}

// endpoint to delete item (hyper)
async fn delete_item_endpoint(
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let barcode = req.uri().path().split('/').last();

    if barcode.is_none() {
        let mut resp = Response::new(full("No barcode"));
        *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
        return Ok(resp);
    }

    let res = delete_item(barcode.unwrap()); // unwrap is safe because we checked it above

    if let Err(err) = res {
        let mut resp = if err == "Item not found" {
            Response::new(full("Item not found"))
        } else {
            Response::new(full(err.clone()))
        };
        *resp.status_mut() = if err == "Item not found" {
            hyper::StatusCode::NOT_FOUND
        } else {
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        };
        return Ok(resp);
    }

    Ok(Response::new(ok()))
}

// endpoint to log an item (hyper)
async fn log_item(
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let barcode = req.uri().path().split('/').last();

    if barcode.is_none() {
        let mut resp = Response::new(full("No barcode"));
        *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
        return Ok(resp);
    }

    let conn = Connection::open(DB_NAME);

    if conn.is_err() {
        let mut resp = Response::new(full(conn.unwrap_err().to_string()));
        *resp.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
        return Ok(resp);
    }

    let rows_affected = conn
        .unwrap() // unwrap is safe because we checked it above
        .execute(
            "UPDATE items SET last_seen = ?1 WHERE barcode = ?2",
            params![Utc::now().timestamp() as u64, barcode],
        );

    match rows_affected {
        Ok(0) => {
            let mut resp = Response::new(full("Item not found"));
            *resp.status_mut() = hyper::StatusCode::NOT_FOUND;
            return Ok(resp);
        }
        Ok(_) => {}
        Err(_) => {
            let mut resp = Response::new(full("Failed to log item"));
            *resp.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
            return Ok(resp);
        }
    }
    Ok(Response::new(ok()))
}

fn cap_at_n(n: usize, s: &str) -> String {
    if s.len() > n {
        format!("{}...", &s[..n])
    } else {
        s.to_string()
    }
}

async fn dispatch(
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let user_agent = match req.headers().get(USER_AGENT) {
        Some(user_agent) => user_agent.to_str().unwrap_or("unknown"),
        None => "unknown",
    };

    print!("[{}] {} {} from {}", chrono::Local::now().format("%Y-%m-%dT%H:%M:%SZ"), req.method(), req.uri().path(), cap_at_n(25, user_agent));
    let res = match req.uri().path() {
        "/new" => new_item(req).await,
        "/all" => all_items(req).await,
        path if path.starts_with("/item/") => item(req).await,
        "/modify" => modify_item_endpoint(req).await,
        path if path.starts_with("/delete/") => delete_item_endpoint(req).await,
        path if path.starts_with("/log/") => log_item(req).await,
        _ => {
            let mut resp = Response::new(full("Not found"));
            *resp.status_mut() = hyper::StatusCode::NOT_FOUND;
            Ok(resp)
        }
    };

    if let Ok(response) = res.as_ref() {
        println!(" -> {}", response.status());
    } else {
        eprintln!(" -> Couldn't process request (unknown error)");
    }

    res.map(|mut resp| {
        // add CORS headers
        resp.headers_mut().insert(
            hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
            hyper::header::HeaderValue::from_static("*"),
        );
        resp
    })
}

fn setup_if_not_exists() {
    let conn = Connection::open(DB_NAME).unwrap();
    let result = conn.execute(
        "CREATE TABLE IF NOT EXISTS items (
            name VARCHAR NOT NULL,
            barcode INTEGER NOT NULL UNIQUE,
            location VARCHAR NOT NULL,
            last_seen TIMESTAMP NOT NULL
        )",
        params![],
    );

    if let Err(e) = result {
        panic!("Failed to create table: {}", e);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    setup_if_not_exists();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            let result = http1::Builder::new()
                .serve_connection(io, service_fn(dispatch))
                .await;

            if let Err(err) = result {
                eprintln!("HTTP/1 Error: {}", err);
            }
        });
    }
}

#[cfg(test)]
fn setup_test_db() {
    let conn = Connection::open("test.db").unwrap();
    let result = conn.execute(
        "CREATE TABLE items (
            name VARCHAR NOT NULL,
            barcode INTEGER NOT NULL UNIQUE,
            location VARCHAR NOT NULL,
            last_seen TIMESTAMP NOT NULL
        )",
        params![],
    );

    if let Err(e) = result {
        if e.to_string().contains("table items already exists") {
            // Table already exists, no need to panic
            return;
        } else {
            // Other errors should still panic
            panic!("Failed to create table: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item() {
        setup_test_db();

        let item = Item::new("item".to_string(), 42, "location".to_string());
        item.save().unwrap();
        let loaded_item = load_item(42).unwrap();
        assert_eq!(item.name, loaded_item.name);
        assert_eq!(item.barcode, loaded_item.barcode);
        assert_eq!(item.location, loaded_item.location);
        assert_eq!(item.last_seen, loaded_item.last_seen);
    }

    #[test]
    fn test_new_and_load() {
        setup_test_db();

        let item = Item::new("item".to_string(), 43, "location".to_string());
        item.save().unwrap();
        let checked_item = load_item(43).unwrap();
        assert_eq!(item.name, checked_item.name);
        assert_eq!(item.barcode, checked_item.barcode);
        assert_eq!(item.location, checked_item.location);
        assert_eq!(item.last_seen, checked_item.last_seen);
    }

    #[test]
    fn test_delete() {
        setup_test_db();

        let item = Item::new("item".to_string(), 44, "location".to_string());
        item.save().unwrap();
        let items_initial_len = load_items().unwrap().len();
        let conn = Connection::open("test.db").unwrap();
        conn.execute("DELETE FROM items WHERE barcode = ?1", params!["44"])
            .unwrap();
        let items = load_items().unwrap();
        assert_eq!(items.len(), items_initial_len - 1);
    }

    #[test]
    fn teardown() {
        // hacky, but just sleep for a bit so the other tests can finish
        std::thread::sleep(std::time::Duration::from_secs(1));

        std::fs::remove_file("test.db").unwrap();
    }
}
