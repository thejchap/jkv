use base64::{engine::general_purpose, Engine as _};
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
pub fn key2path(key: &str) -> String {
    let d = md5::compute(key);
    let b64 = general_purpose::STANDARD_NO_PAD.encode(key);
    format!("{:x}/{:x}/{}", d[0], d[1], b64)
}
