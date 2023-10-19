use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use rocket::{
    fairing::{self, AdHoc},
    get,
    http::{Header, Status},
    put,
    response::Responder,
    Build, Rocket, State,
};
use rocket_db_pools::{sqlx, Connection, Database as SqliteDatabase};
use std::cmp;
/// index of key -> comma separated list of volumes
/// separate from App state because rocket is weird
#[derive(SqliteDatabase)]
#[database("index")]
struct Index(sqlx::SqlitePool);
/// custom responder that includes the redirect
/// as well as custom headers
#[derive(Debug, Clone, Responder)]
#[response(status = 302, content_type = "plain")]
struct VolumeRedirect {
    inner: (),
    key_volumes: Header<'static>,
    location: Header<'static>,
}
/// app state containing config and some other shared stuff
struct App {
    volumes: Vec<String>,
    replicas: u8,
    client: reqwest::Client,
}
/// command line args
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = "a mini-clone of a mini-clone of s3"
)]
struct Args {
    /// comma separated list of volume servers
    #[arg(short, long)]
    volumes: String,
    /// number of replicas to store
    #[arg(short, long, default_value_t = 3)]
    replicas: u8,
}
/// rendezvous hash key -> list of volume servers it should be assigned to
pub fn key2volumes(key: &str, volumes: &[String], k: usize) -> Vec<String> {
    let mut volumes_with_scores: Vec<(md5::Digest, &String)> = volumes
        .iter()
        .map(|v| {
            let d = md5::compute(format!("{}{}", v, key));
            (d, v)
        })
        .collect();
    volumes_with_scores.sort_by(|a, b| a.0.cmp(&b.0));
    volumes_with_scores.truncate(k);
    volumes_with_scores
        .iter()
        .map(|(_, v)| v.to_string())
        .collect()
}
/// key -> path on volume server to control disk layout
pub fn key2path(key: &str) -> String {
    let d = md5::compute(key);
    let b64 = general_purpose::STANDARD_NO_PAD.encode(key);
    format!("{:x}/{:x}/{}", d[0], d[1], b64)
}
/// get the value for a key via a redirect to a volume server containing it
#[get("/<k>")]
async fn get(
    mut db: Connection<Index>,
    app: &State<App>,
    k: &str,
) -> Result<VolumeRedirect, Status> {
    let rec_opt = sqlx::query_scalar!("SELECT value FROM kv WHERE key = ?", k)
        .fetch_optional(&mut **db)
        .await
        .map_err(|e| {
            log::error!("failed to read from index: {}", e);
            Status::InternalServerError
        })?;
    let volumes_str = match rec_opt {
        Some(val) => val.unwrap(),
        None => return Err(Status::NotFound),
    };
    let volumes = volumes_str
        .split(',')
        .map(str::to_string)
        .collect::<Vec<String>>();
    let key_volumes = Header::new("Key-Volumes", volumes_str);
    for volume in volumes {
        let remote_path = key2path(k);
        let url = format!("{}/{}", volume, remote_path);
        let response = app.client.head(&url).send().await;
        if let Ok(resp) = response {
            if resp.status().is_success() {
                return Ok(VolumeRedirect {
                    inner: (),
                    key_volumes,
                    location: Header::new("Location", url),
                });
            }
        }
    }
    Err(Status::NotFound)
}
/// persist the value to volume servers with the configured replication factor
#[put("/<k>", data = "<v>")]
async fn put(
    mut db: Connection<Index>,
    app: &State<App>,
    k: &str,
    v: &str,
) -> Result<Status, Status> {
    if v.is_empty() {
        return Err(Status::BadRequest);
    }
    let key_exists = sqlx::query_scalar!("SELECT 1 FROM kv WHERE key = ?", k)
        .fetch_optional(&mut **db)
        .await
        .map_err(|e| {
            log::error!("failed to read from index: {}", e);
            Status::InternalServerError
        })?;
    if key_exists.is_some() {
        return Err(Status::Conflict);
    }
    let volumes = key2volumes(k, app.volumes.as_slice(), app.replicas as usize);
    let volumes_val = volumes.join(",");
    sqlx::query!("INSERT INTO kv (key, value) VALUES (?, ?)", k, volumes_val)
        .execute(&mut **db)
        .await
        .map_err(|_| {
            log::error!("failed to write to index");
            Status::InternalServerError
        })?;
    // TODO asyncify
    for volume in volumes {
        let remote_path = key2path(k);
        let url = format!("{}/{}", volume, remote_path);
        let response = app.client.put(url).body(v.to_string()).send().await;
        if response.is_err() {
            log::error!("put error: {:?}", response.err().unwrap());
            return Err(Status::InternalServerError);
        }
    }
    Ok(Status::Created)
}
/// set up the index k/v table. using sqlite/sqlx because the
/// k/v options in rust are kind of all over the place, don't have great async support (rocket is async)
async fn create_table(rkt: Rocket<Build>) -> fairing::Result {
    match Index::fetch(&rkt) {
        Some(db) => {
            match sqlx::query!(
                r#"
CREATE TABLE IF NOT EXISTS
kv (key TEXT UNIQUE, value TEXT)
"#
            )
            .execute(&db.0)
            .await
            {
                Ok(_) => Ok(rkt),
                Err(e) => {
                    log::error!("failed to create table: {}", e);
                    Err(rkt)
                }
            }
        }
        None => Err(rkt),
    }
}
/// entrypoint
#[rocket::launch]
fn server() -> _ {
    env_logger::init();
    let args = Args::parse();
    let volumes: Vec<String> = args.volumes.split(',').map(str::to_string).collect();
    let replicas = cmp::min(args.replicas, volumes.len() as u8);
    let client = reqwest::Client::new();
    let app = App {
        volumes,
        replicas,
        client,
    };
    log::warn!("volumes: {:?}. replicas: {:?}:", app.volumes, app.replicas);
    rocket::build()
        .manage(app)
        .attach(Index::init())
        .attach(AdHoc::try_on_ignite("create_table", create_table))
        .mount("/", rocket::routes![get, put])
}
