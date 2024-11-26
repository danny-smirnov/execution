use std::fs::{self, File};
use std::io::BufReader;
use clickhouse::{error::Result, sql::Identifier, Client, Compression, Row};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Row)]
struct Event {
    local_unique_id: i64,
    venue_timestamp: i64,
    gate_timestamp: i64,
    event_type: String,
    product: String,
    id1: Option<u64>,
    id2: Option<u64>,
    ask_not_bid: Option<bool>,
    buy_not_sell: Option<bool>,
    price: String,
    quantity: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let table_name = "marketDataUnprocessed";
    let client = Client::default()
        .with_url("http://127.0.1.1:8123")
        .with_user("default")
        .with_database("default")
        .with_compression(Compression::None);

    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS ? (
                local_unique_id Int64,
                venue_timestamp DateTime64(3, 'UTC'),
                gate_timestamp  DateTime64(9, 'UTC'),
                event_type      String,
                product         String,
                id1             Nullable(UInt64),
                id2             Nullable(UInt64),
                ask_not_bid     Nullable(Bool),
                buy_not_sell    Nullable(Bool),
                price           String,
                quantity        String
            )
            ENGINE = MergeTree
            ORDER BY (local_unique_id, venue_timestamp)
            PARTITION BY toYYYYMMDD(gate_timestamp)
            "#,
        )
        .bind(Identifier(table_name))
        .execute()
        .await?;

    let dir_path = "/srv/storage/marketdata/";
    let files = fs::read_dir(dir_path).expect("Unable to read directory");

    for entry in files {
        let entry = entry.expect("Unable to access file entry");
        let file_path = entry.path();

        if file_path.is_file() && file_path.extension().and_then(|ext| ext.to_str()) == Some("bin") {
            println!("Processing file: {:?}", file_path);

            let file = File::open(&file_path).expect("Unable to open file");
            let metadata = file.metadata().expect("Unable to get file metadata");
            let total_size = metadata.len();

            let mut reader = BufReader::new(file);
            let mut insert = client.insert(table_name)?;

            let pb = ProgressBar::new(total_size);
            pb.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("##-"),
            );

            loop {
                match bincode::deserialize_from::<_, Event>(&mut reader) {
                    Ok(event) => {
                        insert.write(&event).await?;
                        pb.inc(std::mem::size_of::<Event>() as u64);
                    }
                    Err(err) => match *err {
                        bincode::ErrorKind::Io(ref io_err)
                            if io_err.kind() == std::io::ErrorKind::UnexpectedEof =>
                        {
                            println!("End of file reached: {:?}", file_path);
                            break;
                        }
                        _ => {
                            eprintln!("Error reading file {:?}: {}", file_path, err);
                            break;
                        }
                    },
                }
            }
            insert.end().await?;
        }
    }

    println!("All files have been processed.");
    Ok(())
}
