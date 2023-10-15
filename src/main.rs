#[macro_use]
extern crate rocket;
extern crate log;
use clap::Parser;
use jkv::{key2path, key2volumes};
use rocket::{
    http::{Header, Status},
    response::Responder,
    State,
};
use std::{cmp, collections::HashMap, vec};
use tokio::sync::RwLock;
#[derive(Debug, Clone)]
struct Record {
    volumes: Vec<String>,
    value: String,
}
#[derive(Responder)]
#[response(status = 302, content_type = "plain")]
struct VolumeRedirect {
    inner: String,
    key_volumes: Header<'static>,
    location: Header<'static>,
}
#[derive(Debug)]
struct App {
    db: RwLock<HashMap<String, Record>>,
    volumes: Vec<String>,
    replicas: u8,
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    volumes: String,
    #[arg(short, long, default_value_t = 3)]
    replicas: u8,
}
#[get("/<k>")]
async fn get(app: &State<App>, k: &str) -> Result<VolumeRedirect, Status> {
    let db = app.db.read().await;
    let record = db.get(k);
    if record.is_none() {
        return Err(Status::NotFound);
    }
    let client = reqwest::Client::new();
    for volume in &record.unwrap().volumes {
        let remote_path = key2path(k);
        let url = format!("{}/{}", volume, remote_path);
        let res = client.head(&url).send().await;
        if res.is_ok() && res.unwrap().status().is_success() {
            return Ok(VolumeRedirect {
                inner: "".to_string(),
                key_volumes: Header::new("Key-Volumes", record.unwrap().volumes.join(",")),
                location: Header::new("Location", url),
            });
        }
    }
    Err(Status::NotFound)
}
#[put("/<k>", data = "<v>")]
async fn put(app: &State<App>, k: &str, v: &str) -> Status {
    if v.is_empty() {
        return Status::BadRequest;
    }
    let mut db = app.db.write().await;
    if db.contains_key(k) {
        return Status::Conflict;
    }
    let volumes = key2volumes(k, app.volumes.as_slice(), app.replicas as usize);
    let r = Record {
        volumes,
        value: v.to_string(),
    };
    let client = reqwest::Client::new();
    for volume in &r.volumes {
        let remote_path = key2path(k);
        let url = format!("{}/{}", volume, remote_path);
        dbg!(&url);
        let res = client.put(url).body(r.value.to_string()).send().await;
        if res.is_err() {
            error!("put error: {:?}", res.err().unwrap());
            return Status::InternalServerError;
        }
    }
    db.insert(k.to_string(), r);
    Status::Created
}
#[delete("/<k>")]
async fn delete(app: &State<App>, k: &str) -> Status {
    let mut db = app.db.write().await;
    match db.remove(k) {
        Some(_) => Status::NoContent,
        None => Status::NotFound,
    }
}
#[launch]
fn server() -> _ {
    env_logger::init();
    let args = Args::parse();
    let volumes: Vec<String> = args.volumes.split(',').map(str::to_string).collect();
    let replicas = cmp::min(args.replicas, volumes.len() as u8);
    let db = RwLock::new(HashMap::new());
    let app = App {
        db,
        volumes,
        replicas,
    };
    warn!("volumes: {:?}. replicas: {:?}:", app.volumes, app.replicas);
    rocket::build()
        .manage(app)
        .mount("/", routes![get, put, delete])
}
