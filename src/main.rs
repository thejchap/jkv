#[macro_use]
extern crate rocket;
extern crate log;
use clap::Parser;
use rocket::{
    http::{Header, Status},
    response::Responder,
    State,
};
use std::{collections::HashMap, sync::RwLock, vec};
#[derive(Debug)]
struct Record {
    volumes: Vec<String>,
    value: String,
}
#[derive(Responder)]
#[response(status = 200, content_type = "plain")]
struct RecordResponse {
    inner: String,
    key_volumes: Header<'static>,
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
fn get(app: &State<App>, k: &str) -> Result<RecordResponse, Status> {
    let db = app.db.read().unwrap();
    match db.get(k) {
        Some(r) => Ok(RecordResponse {
            inner: r.value.to_string(),
            key_volumes: Header::new("Key-Volumes", r.volumes[0..app.replicas as usize].join(",")),
        }),
        None => Err(Status::NotFound),
    }
}
#[put("/<k>", data = "<v>")]
fn put(app: &State<App>, k: &str, v: &str) -> Status {
    if v.is_empty() {
        return Status::BadRequest;
    }
    let mut db = app.db.write().unwrap();
    if db.contains_key(k) {
        return Status::Conflict;
    }
    let r = Record {
        volumes: app.volumes.clone(),
        value: v.to_string(),
    };
    db.insert(k.to_string(), r);
    Status::Created
}
#[delete("/<k>")]
fn delete(app: &State<App>, k: &str) -> Status {
    let mut db = app.db.write().unwrap();
    match db.remove(k) {
        Some(_) => Status::NoContent,
        None => Status::NotFound,
    }
}
#[launch]
fn server() -> _ {
    env_logger::init();
    let args = Args::parse();
    let volumes = args.volumes.split(',').map(str::to_string).collect();
    let app = App {
        db: RwLock::new(HashMap::new()),
        volumes,
        replicas: args.replicas,
    };
    info!("volumes: {:?}. replicas: {:?}:", app.volumes, app.replicas);
    rocket::build()
        .manage(app)
        .mount("/", routes![get, put, delete])
}
