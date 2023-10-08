#[macro_use]
extern crate rocket;
use rocket::{http::Status, State};
use std::{collections::HashMap, sync::RwLock, vec};
struct App {
    db: RwLock<HashMap<String, String>>,
    volumes: Vec<String>,
}
impl App {
    fn new() -> Self {
        App {
            db: RwLock::new(HashMap::new()),
            volumes: vec![],
        }
    }
}
#[get("/<key>")]
fn get(app: &State<App>, key: &str) -> Result<String, Status> {
    let db = app.db.read().unwrap();
    dbg!(app.volumes.to_owned());
    db.get(key)
        .map(|val| val.to_string())
        .ok_or(Status::NotFound)
}
#[put("/<key>", data = "<val>")]
fn put(app: &State<App>, key: &str, val: &str) -> Status {
    let mut db = app.db.write().unwrap();
    if db.contains_key(key) {
        return Status::Conflict;
    }
    db.insert(key.to_string(), val.to_string());
    Status::Created
}
#[launch]
fn server() -> _ {
    rocket::build()
        .manage(App::new())
        .mount("/", routes![put, get])
}
