use std::io::{Read, Write};

use chrono::TimeZone;
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
/// terminal interface to server in ../server
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

lazy_static! {
    static ref SERVER: Mutex<OnceCell<String>> = Mutex::new(OnceCell::new());
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    name: String,
    barcode: u64,
    location: String,
}

async fn new_item(item: Item) -> Result<u16, reqwest::Error> {
    let client = reqwest::Client::new();

    let res = client
        .post(format!(
            "{}/new",
            SERVER.lock().unwrap().get().expect("Server not set")
        ))
        .body(serde_json::to_string(&item).expect("Failed to serialize item"));

    Ok(res.send().await?.status().as_u16())
}

async fn modify_item(item: Item) -> Result<u16, reqwest::Error> {
    let client = reqwest::Client::new();

    let res = client
        .post(format!(
            "{}/modify",
            SERVER.lock().unwrap().get().expect("Server not set")
        ))
        .body(serde_json::to_string(&item).expect("Failed to serialize item"));

    Ok(res.send().await?.status().as_u16())
}

async fn delete_item(barcode: u64) -> Result<u16, reqwest::Error> {
    let client = reqwest::Client::new();

    let res = client.get(format!(
        "{}/delete/{}",
        SERVER.lock().unwrap().get().expect("Server not set"),
        barcode
    ));

    Ok(res.send().await?.status().as_u16())
}

async fn get_all_items() -> Result<u16, reqwest::Error> {
    let client = reqwest::Client::new();

    let res = client.get(format!(
        "{}/all",
        SERVER.lock().unwrap().get().expect("Server not set")
    ));

    let items = res.send().await?;

    if items.status().as_u16() != 200 {
        return Ok(items.status().as_u16());
    }

    let items = items.text().await?;

    let actual_items = serde_json::from_str::<serde_json::Value>(&items)
        .expect("Failed to deserialize items")
        .clone();

    for item in actual_items.as_array().expect("Failed to get items") {
        #[allow(deprecated)]
        let last_seen = chrono::NaiveDateTime::from_timestamp(
            item["last_seen"]
                .as_i64()
                .expect("Failed to parse last_seen"),
            0,
        );
        let local_last_seen = chrono::Local.from_utc_datetime(&last_seen);
        let formatted_last_seen = local_last_seen.format("%Y-%m-%d %H:%M:%S").to_string();
        println!(
            "{}: {} @ {}, last seen {}",
            item["barcode"], item["name"], item["location"], formatted_last_seen
        );
    }

    println!("Retrieved {} items", actual_items.as_array().expect("Failed to get items").len());

    Ok(200)
}

async fn see_item(barcode: u64) -> Result<u16, reqwest::Error> {
    let client = reqwest::Client::new();

    let res = client.get(format!(
        "{}/item/{}",
        SERVER.lock().unwrap().get().expect("Server not set"),
        barcode
    ));

    let item = res.send().await?;

    if item.status().as_u16() != 200 {
        return Ok(item.status().as_u16());
    }

    let item = item.text().await?;

    let actual_item = serde_json::from_str::<serde_json::Value>(&item)
        .expect("Failed to deserialize item")
        .clone();

    #[allow(deprecated)]
    let last_seen = chrono::NaiveDateTime::from_timestamp(
        actual_item["last_seen"]
            .as_i64()
            .expect("Failed to parse last_seen"),
        0,
    );
    let local_last_seen = chrono::Local.from_utc_datetime(&last_seen);
    let formatted_last_seen = local_last_seen.format("%Y-%m-%d %H:%M:%S").to_string();
    println!(
        "{}: {} @ {}, last seen {}",
        actual_item["barcode"], actual_item["name"], actual_item["location"], formatted_last_seen
    );

    Ok(200)
}

async fn log_item(barcode: u64) -> Result<u16, reqwest::Error> {
    let client = reqwest::Client::new();

    let res = client.get(format!(
        "{}/log/{}",
        SERVER.lock().unwrap().get().expect("Server not set"),
        barcode
    ));

    Ok(res.send().await?.status().as_u16())
}

fn process_new_item(barcode: u64) -> Item {
    // first, barcode will be inputted followed by \n, followed by a location hotkey, then a name

    let mut location = String::new();
    flush_print!("new>{}>location> ", barcode);
    std::io::stdin()
        .read_line(&mut location)
        .expect("Failed to read input");

    let actual_location = match location.trim() {
        "l" => "Levi Fox Hall Tech Box",
        "d" => "Drama Studio Tech Box",
        "r" => "Rig",
        "s" => "Storage outside Levi Fox Hall Tech Box",
        _ => location.trim(),
    };

    let mut name = String::new();
    flush_print!("new>{}>name> ", barcode);
    std::io::stdin()
        .read_line(&mut name)
        .expect("Failed to read input");

    Item {
        name: name.trim().to_string(),
        barcode: barcode,
        location: actual_location.to_string(),
    }
}

fn process_modify_item(barcode: u64) -> Item {
    let mut location = String::new();
    flush_print!("modify>{}>location> ", barcode);
    std::io::stdin()
        .read_line(&mut location)
        .expect("Failed to read input");

    let actual_location = match location.trim() {
        "l" => "Levi Fox Hall Tech Box",
        "d" => "Drama Studio Tech Box",
        "r" => "Rig",
        "s" => "Storage outside Levi Fox Hall Tech Box",
        _ => location.trim(),
    };

    let mut name = String::new();
    flush_print!("modify>{}>name> ", barcode);
    std::io::stdin()
        .read_line(&mut name)
        .expect("Failed to read input");

    Item {
        name: name.trim().to_string(),
        barcode: barcode,
        location: actual_location.to_string(),
    }
}

// define print macro with flush
mod macros {
    #[macro_export]
    macro_rules! flush_print {
    ($($arg:tt)*) => {
        print!($($arg)*);
        std::io::stdout().flush().expect("Failed to flush buffer");
    };
}
}

fn get_args(s: String) -> Vec<u64> {
    s.split_whitespace()
        .skip(1) // skip the command
        .map(|x| x.parse().expect("Failed to parse"))
        .collect()
}

fn load_server_ip() {
    // server ip will probably be in `barcode.cfg`
    // if it is not, prompt the user for the server ip
    // and write it to `barcode.cfg`
    if std::fs::exists("barcode.cfg").unwrap() {
        let mut file = std::fs::File::open("barcode.cfg").expect("Failed to open barcode.cfg");
        let mut server = String::new();
        file.read_to_string(&mut server)
            .expect("Failed to read barcode.cfg");
        SERVER
            .lock()
            .unwrap()
            .set(server.trim().to_string())
            .expect("Failed to set server");
    } else {
        let mut server = String::new();
        flush_print!("server addr> ");

        std::io::stdin()
            .read_line(&mut server)
            .expect("Failed to read input");

        let server = match server {
            s if s.starts_with("http://") => s,
            s if s.starts_with("https://") => s.replace("https://", "http://"),
            s => format!("http://{}", s),
        };

        SERVER
            .lock()
            .unwrap()
            .set(server.trim().to_string())
            .expect("Failed to set server");

        let mut file = std::fs::File::create("barcode.cfg").expect("Failed to create barcode.cfg");
        file.write_all(server.as_bytes())
            .expect("Failed to write to barcode.cfg");
    }
}

#[tokio::main]
async fn main() {
    load_server_ip();

    let mut input = String::new();

    loop {
        flush_print!("> ");
        input.clear();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
        match input
            .trim()
            .split_whitespace()
            .next()
            .expect("Failed to parse command")
        {
            "new" => {
                let args = get_args(input.to_string());
                for barcode in args.clone() {
                    match new_item(process_new_item(barcode)).await {
                        Ok(status) if status == 200 => {},
                        Ok(status) => eprintln!("Failed to create item with barcode {}: HTTP {}", barcode, status),
                        Err(e) => eprintln!("Error creating item with barcode {}: {}", barcode, e),
                    }
                }
                println!("Created {} items", args.len());
            }
            "modify" => {
                let args = get_args(input.to_string());
                for barcode in args.clone() {
                    match modify_item(process_modify_item(barcode)).await {
                        Ok(status) if status == 200 => {},
                        Ok(status) => eprintln!("Failed to modify item with barcode {}: HTTP {}", barcode, status),
                        Err(e) => eprintln!("Error modifying item with barcode {}: {}", barcode, e),
                    }
                }
                println!("Modified {} items", args.len());
            }
            "delete" => {
                let args = get_args(input.to_string());
                for barcode in args.clone() {
                    match delete_item(barcode).await {
                        Ok(status) if status == 200 => {},
                        Ok(status) => eprintln!("Failed to delete item with barcode {}: HTTP {}", barcode, status),
                        Err(e) => eprintln!("Error deleting item with barcode {}: {}", barcode, e),
                    }
                }
                println!("Deleted {} items", args.len());
            }
            "log" => {
                let args = get_args(input.to_string());
                for barcode in args.clone() {
                    match log_item(barcode).await {
                        Ok(status) if status == 200 => {},
                        Ok(status) => eprintln!("Failed to log item with barcode {}: HTTP {}", barcode, status),
                        Err(e) => eprintln!("Error logging item with barcode {}: {}", barcode, e),
                    }
                }
                println!("Logged {} items", args.len());
            }
            "all" => {
                match get_all_items().await {
                    Ok(status) if status == 200 => {}, // printing handled by get_all_items
                    Ok(status) => eprintln!("Failed to retrieve all items: HTTP {}", status),
                    Err(e) => eprintln!("Error retrieving all items: {}", e),
                }
            }
            "see" => {
                let args = get_args(input.to_string());
                for barcode in args.clone() {
                    match see_item(barcode).await {
                        Ok(status) if status == 200 => {},
                        Ok(status) => eprintln!("Failed to retrieve item with barcode {}: HTTP {}", barcode, status),
                        Err(e) => eprintln!("Error retrieving item with barcode {}: {}", barcode, e),
                    }
                }
                println!("Retrieved {} items", args.len());
            }
            "server" => {
                // change the server ip
                let mut server = String::new();
                flush_print!("server addr> ");
                std::io::stdin()
                    .read_line(&mut server)
                    .expect("Failed to read input");

                SERVER.lock().unwrap().take();

                SERVER
                    .lock()
                    .unwrap()
                    .set(server.trim().to_string())
                    .expect("Failed to set server");

                let mut file =
                    std::fs::File::create("barcode.cfg").expect("Failed to create barcode.cfg");
                file.write_all(server.as_bytes())
                    .expect("Failed to write to barcode.cfg");
            }
            "quit" => break,
            _ => {
                println!(
                    "
Commands:
new <barcode1> <barcode2> ... - create new item
modify <barcode1> <barcode2> ... - modify item
delete <barcode1> <barcode2> ... - delete item
log <barcode1> <barcode2> ... - see item
all - get all items
see <barcode1> <barcode2> ... - get item
server - change server ip
quit - quit

server will be written to and read from barcode.cfg
"
                );
            }
        }
    }
}
