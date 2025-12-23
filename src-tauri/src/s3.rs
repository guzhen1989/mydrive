// Reqwest + SigV4 backend with upload-id persistence and download-range-append support
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use directories::ProjectDirs;
use hmac::{Hmac, Mac};
use md5;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::{Client, Method};
use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::fs::{File as TokioFile, OpenOptions};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize, Debug, Clone)]
pub struct ConnectConfig {
  pub endpoint: String,
  pub port: Option<u16>,
  pub use_ssl: Option<bool>,
  pub access_key: Option<String>,
  pub secret_key: Option<String>,
  // optional base64 SSE-C key (256-bit base64)
  pub sse_c_key_b64: Option<String>,
  // optional region (default us-east-1)
  pub region: Option<String>,
}

#[derive(Serialize)]
pub struct BucketInfo {
  pub Name: String,
  pub CreationDate: String,
}

#[derive(Serialize)]
pub struct ObjectInfo {
  pub Key: String,
  pub Size: i64,
  pub LastModified: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CompletedPartResp {
  pub PartNumber: i32,
  pub ETag: String,
}

// persisted upload record
#[derive(Serialize, Deserialize, Clone)]
pub struct UploadRecord {
  pub bucket: String,
  pub key: String,
  pub upload_id: String,
  pub parts: Vec<CompletedPartResp>,
}

#[derive(Clone)]
pub struct S3ClientWrapper {
  client: Client,
  cfg: ConnectConfig,
  base_url: String, // e.g. http://127.0.0.1:9000
  region: String,
}

impl S3ClientWrapper {
  pub async fn new(cfg: ConnectConfig) -> Result<Self> {
    let scheme = if cfg.use_ssl.unwrap_or(false) { "https" } else { "http" };
    let port = cfg.port.unwrap_or(9000);
    let base_url = format!("{}://{}:{}", scheme, cfg.endpoint, port);
    let client = Client::builder()
      .danger_accept_invalid_certs(false)
      .build()?;
    let region = cfg.region.clone().unwrap_or_else(|| "us-east-1".to_string());
    Ok(Self { client, cfg, base_url, region })
  }

  // -------------------------
  // Helper: SigV4 signing
  // -------------------------
  fn sign_request(
    &self,
    method: &Method,
    host: &str,
    canonical_uri: &str,
    query: &BTreeMap<String, String>,
    headers: &BTreeMap<String, String>,
    payload: &[u8],
    service: &str,
  ) -> Result<BTreeMap<String, String>> {
    let ak = self.cfg.access_key.as_ref().ok_or_else(|| anyhow!("missing access key"))?;
    let sk = self.cfg.secret_key.as_ref().ok_or_else(|| anyhow!("missing secret key"))?;
    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();

    let mut qvec: Vec<String> = Vec::new();
    for (k, v) in query.iter() {
      qvec.push(format!("{}={}", percent_encode(k), percent_encode(v)));
    }
    let canonical_query = qvec.join("&");

    let mut chdrs = BTreeMap::new();
    chdrs.insert("host".to_string(), host.to_string());
    chdrs.insert("x-amz-date".to_string(), amz_date.clone());
    for (k, v) in headers.iter() {
      chdrs.insert(k.to_lowercase(), v.clone());
    }
    let payload_hash = hex::encode(Sha256::digest(payload));
    chdrs.insert("x-amz-content-sha256".to_string(), payload_hash.clone());

    let mut canonical_headers = String::new();
    let mut signed_headers_vec: Vec<String> = Vec::new();
    for (k, v) in chdrs.iter() {
      canonical_headers.push_str(&format!("{}:{}\n", k, v.trim()));
      signed_headers_vec.push(k.clone());
    }
    let signed_headers = signed_headers_vec.join(";");

    let canonical_request = format!(
      "{}\n{}\n{}\n{}\n{}\n{}",
      method.as_str(),
      canonical_uri,
      canonical_query,
      canonical_headers,
      signed_headers,
      payload_hash
    );

    let algorithm = "AWS4-HMAC-SHA256";
    let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, self.region, service);
    let hash_can_req = hex::encode(Sha256::digest(canonical_request.as_bytes()));
    let string_to_sign = format!("{}\n{}\n{}\n{}", algorithm, amz_date, credential_scope, hash_can_req);

    let k_date = hmac_sha256(format!("AWS4{}", sk).as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, self.region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    let authorization = format!(
      "{} Credential={}/{}, SignedHeaders={}, Signature={}",
      algorithm, ak, credential_scope, signed_headers, signature
    );

    let mut final_headers = BTreeMap::new();
    final_headers.insert("authorization".to_string(), authorization);
    final_headers.insert("x-amz-date".to_string(), amz_date);
    final_headers.insert("x-amz-content-sha256".to_string(), payload_hash);
    for (k, v) in headers.iter() {
      final_headers.insert(k.to_lowercase(), v.clone());
    }
    Ok(final_headers)
  }

  // -------------------------
  // Helpers: low-level HTTP
  // -------------------------
  async fn do_request(
    &self,
    method: Method,
    url_path: &str,
    query: BTreeMap<String, String>,
    mut headers: BTreeMap<String, String>,
    body: Vec<u8>,
  ) -> Result<reqwest::Response> {
    let url = format!("{}{}", self.base_url, url_path);
    let url_parsed = reqwest::Url::parse(&url)?;
    let host = match url_parsed.host_str() {
      Some(h) => {
        if let Some(p) = url_parsed.port() {
          format!("{}:{}", h, p)
        } else {
          h.to_string()
        }
      }
      None => return Err(anyhow!("invalid host in url")),
    };

    let signed = self.sign_request(&method, &host, url_parsed.path(), &query, &headers, &body, "s3")?;
    let mut req = self.client.request(method.clone(), url.clone());
    if !query.is_empty() {
      // attach query pairs
      let mut pairs: Vec<(String, String)> = Vec::new();
      for (k, v) in query.iter() { pairs.push((k.clone(), v.clone())); }
      req = req.query(&pairs);
    }
    for (k, v) in signed.iter() {
      req = req.header(k, v);
    }
    if !body.is_empty() {
      req = req.body(body);
    }
    let resp = req.send().await?;
    Ok(resp)
  }

  // -------------------------
  // Persistence for upload records
  // -------------------------
  fn uploads_db_path(&self) -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "example", "minio-tauri").ok_or_else(|| anyhow!("cannot get project dirs"))?;
    let cache = proj_dirs.cache_dir();
    std::fs::create_dir_all(cache)?;
    let mut p = PathBuf::from(cache);
    p.push("uploads.json");
    Ok(p)
  }

  async fn load_uploads_map(&self) -> Result<HashMap<String, UploadRecord>> {
    let p = self.uploads_db_path()?;
    if !tokio::fs::metadata(&p).await.is_ok() {
      return Ok(HashMap::new());
    }
    let s = tokio::fs::read_to_string(&p).await?;
    let m: HashMap<String, UploadRecord> = serde_json::from_str(&s).unwrap_or_default();
    Ok(m)
  }

  async fn save_uploads_map(&self, map: &HashMap<String, UploadRecord>) -> Result<()> {
    let p = self.uploads_db_path()?;
    let tmp = p.with_extension("tmp");
    let s = serde_json::to_string_pretty(map)?;
    tokio::fs::write(&tmp, s).await?;
    tokio::fs::rename(&tmp, &p).await?;
    Ok(())
  }

  // key for map: "{bucket}|{key}"
  fn map_key(bucket: &str, key: &str) -> String {
    format!("{}|{}", bucket, key)
  }

  pub async fn save_upload_record(&self, bucket: &str, key: &str, upload_id: &str) -> Result<()> {
    let mut map = self.load_uploads_map().await?;
    let rec = UploadRecord {
      bucket: bucket.to_string(),
      key: key.to_string(),
      upload_id: upload_id.to_string(),
      parts: Vec::new(),
    };
    map.insert(Self::map_key(bucket, key), rec);
    self.save_uploads_map(&map).await?;
    Ok(())
  }

  pub async fn append_part_record(&self, bucket: &str, key: &str, upload_id: &str, part_number: i32, etag: &str) -> Result<()> {
    let mut map = self.load_uploads_map().await?;
    let mk = Self::map_key(bucket, key);
    if let Some(mut rec) = map.remove(&mk) {
      // ensure upload_id matches; if not, replace
      if rec.upload_id != upload_id {
        rec.upload_id = upload_id.to_string();
        rec.parts.clear();
      }
      // add or replace part
      rec.parts.retain(|p| p.PartNumber != part_number);
      rec.parts.push(CompletedPartResp { PartNumber: part_number, ETag: etag.to_string() });
      map.insert(mk, rec);
    } else {
      let rec = UploadRecord {
        bucket: bucket.to_string(),
        key: key.to_string(),
        upload_id: upload_id.to_string(),
        parts: vec![CompletedPartResp { PartNumber: part_number, ETag: etag.to_string() }],
      };
      map.insert(mk, rec);
    }
    self.save_uploads_map(&map).await?;
    Ok(())
  }

  pub async fn get_persisted_upload(&self, bucket: &str, key: &str) -> Result<Option<UploadRecord>> {
    let map = self.load_uploads_map().await?;
    Ok(map.get(&Self::map_key(bucket, key)).cloned())
  }

  pub async fn list_persisted_uploads(&self) -> Result<Vec<UploadRecord>> {
    let map = self.load_uploads_map().await?;
    Ok(map.values().cloned().collect())
  }

  pub async fn remove_upload_record(&self, bucket: &str, key: &str) -> Result<()> {
    let mut map = self.load_uploads_map().await?;
    map.remove(&Self::map_key(bucket, key));
    self.save_uploads_map(&map).await?;
    Ok(())
  }

  // -------------------------
  // High-level S3 operations (same as before, with persistence where needed)
  // -------------------------
  pub async fn list_buckets(&self) -> Result<Vec<BucketInfo>> {
    let path = "/".to_string();
    let query = BTreeMap::new();
    let headers = BTreeMap::new();
    let body: Vec<u8> = Vec::new();
    let resp = self.do_request(Method::GET, &path, query, headers, body).await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
      return Err(anyhow!("list_buckets error {}: {}", status, text));
    }
    // parse XML
    let mut rdr = Reader::from_str(&text);
    rdr.trim_text(true);
    let mut buf = Vec::new();
    let mut cur: Option<String> = None;
    let mut name: Option<String> = None;
    let mut cdate: Option<String> = None;
    let mut out = Vec::new();
    loop {
      match rdr.read_event_into(&mut buf) {
        Ok(Event::Start(ref e)) => {
          cur = Some(String::from_utf8_lossy(e.name().as_ref()).to_string());
        }
        Ok(Event::Text(e)) => {
          if let Some(ref tag) = cur {
            match tag.as_str() {
              "Name" => { name = Some(e.unescape().unwrap_or_default().to_string()); }
              "CreationDate" => { cdate = Some(e.unescape().unwrap_or_default().to_string()); }
              _ => {}
            }
          }
        }
        Ok(Event::End(ref e)) => {
          if e.name().as_ref() == b"Bucket" {
            if let Some(n) = name.take() {
              out.push(BucketInfo { Name: n, CreationDate: cdate.take().unwrap_or_default() });
            }
          }
          cur = None;
        }
        Ok(Event::Eof) => break,
        Err(e) => return Err(anyhow!("xml parse error: {}", e)),
        _ => {}
      }
      buf.clear();
    }
    Ok(out)
  }

  pub async fn list_objects(&self, bucket: &str) -> Result<Vec<ObjectInfo>> {
    let path = format!("/{}{}", percent_encode(bucket), "?list-type=2");
    let query = BTreeMap::new();
    let headers = BTreeMap::new();
    let resp = self.do_request(Method::GET, &path, query, headers, Vec::new()).await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
      return Err(anyhow!("list_objects error {}: {}", status, text));
    }
    let mut rdr = Reader::from_str(&text);
    rdr.trim_text(true);
    let mut buf = Vec::new();
    let mut cur: Option<String> = None;
    let mut key: Option<String> = None;
    let mut size: Option<i64> = None;
    let mut lm: Option<String> = None;
    let mut out = Vec::new();
    loop {
      match rdr.read_event_into(&mut buf) {
        Ok(Event::Start(ref e)) => { cur = Some(String::from_utf8_lossy(e.name().as_ref()).to_string()); }
        Ok(Event::Text(e)) => {
          if let Some(ref tag) = cur {
            match tag.as_str() {
              "Key" => key = Some(e.unescape().unwrap_or_default().to_string()),
              "Size" => size = Some(e.unescape().unwrap_or_default().parse::<i64>().unwrap_or(0)),
              "LastModified" => lm = Some(e.unescape().unwrap_or_default().to_string()),
              _ => {}
            }
          }
        }
        Ok(Event::End(ref e)) => {
          if e.name().as_ref() == b"Contents" {
            if let Some(k) = key.take() {
              out.push(ObjectInfo { Key: k, Size: size.take().unwrap_or(0), LastModified: lm.take().unwrap_or_default() });
            }
          }
          cur = None;
        }
        Ok(Event::Eof) => break,
        Err(e) => return Err(anyhow!("xml parse error: {}", e)),
        _ => {}
      }
      buf.clear();
    }
    Ok(out)
  }

  pub async fn start_multipart_upload(&self, bucket: &str, key: &str) -> Result<String> {
    let encoded_key = encode_object_key(key);
    let path = format!("/{}/{}?uploads", percent_encode(bucket), encoded_key);
    let query = BTreeMap::new();
    let mut headers = BTreeMap::new();
    if let Some(kb64) = &self.cfg.sse_c_key_b64 {
      headers.insert("x-amz-server-side-encryption-customer-algorithm".to_string(), "AES256".to_string());
      headers.insert("x-amz-server-side-encryption-customer-key".to_string(), kb64.clone());
      if let Ok(raw) = general_purpose::STANDARD.decode(kb64) {
        let md5sum = md5::compute(&raw);
        headers.insert("x-amz-server-side-encryption-customer-key-md5".to_string(), base64::engine::general_purpose::STANDARD.encode(md5sum.0));
      }
    }
    let resp = self.do_request(Method::POST, &path, query, headers, Vec::new()).await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
      return Err(anyhow!("start_multipart_upload error {}: {}", status, text));
    }
    // parse UploadId
    let mut rdr = Reader::from_str(&text);
    rdr.trim_text(true);
    let mut buf = Vec::new();
    let mut cur: Option<String> = None;
    let mut upload_id: Option<String> = None;
    loop {
      match rdr.read_event_into(&mut buf) {
        Ok(Event::Start(ref e)) => { cur = Some(String::from_utf8_lossy(e.name().as_ref()).to_string()); }
        Ok(Event::Text(e)) => {
          if let Some(ref tag) = cur {
            if tag == "UploadId" {
              upload_id = Some(e.unescape().unwrap_or_default().to_string());
            }
          }
        }
        Ok(Event::Eof) => break,
        _ => {}
      }
      buf.clear();
    }
    Ok(upload_id.unwrap_or_else(|| Uuid::new_v4().to_string()))
  }

  pub async fn list_parts(&self, bucket: &str, key: &str, upload_id: &str) -> Result<Vec<CompletedPartResp>> {
    let encoded_key = encode_object_key(key);
    let path = format!("/{}/{}?uploadId={}", percent_encode(bucket), encoded_key, percent_encode(upload_id));
    let query = BTreeMap::new();
    let headers = BTreeMap::new();
    let resp = self.do_request(Method::GET, &path, query, headers, Vec::new()).await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
      return Err(anyhow!("list_parts error {}: {}", status, text));
    }
    let mut rdr = Reader::from_str(&text);
    rdr.trim_text(true);
    let mut buf = Vec::new();
    let mut cur: Option<String> = None;
    let mut partnum: Option<i32> = None;
    let mut etag: Option<String> = None;
    let mut out = Vec::new();
    loop {
      match rdr.read_event_into(&mut buf) {
        Ok(Event::Start(ref e)) => { cur = Some(String::from_utf8_lossy(e.name().as_ref()).to_string()); }
        Ok(Event::Text(e)) => {
          if let Some(ref tag) = cur {
            match tag.as_str() {
              "PartNumber" => partnum = Some(e.unescape().unwrap_or_default().parse::<i32>().unwrap_or_default()),
              "ETag" => etag = Some(e.unescape().unwrap_or_default().to_string()),
              _ => {}
            }
          }
        }
        Ok(Event::End(ref e)) => {
          if e.name().as_ref() == b"Part" {
            if let Some(pn) = partnum.take() {
              out.push(CompletedPartResp { PartNumber: pn, ETag: etag.take().unwrap_or_default() });
            }
          }
          cur = None;
        }
        Ok(Event::Eof) => break,
        Err(e) => return Err(anyhow!("xml parse error: {}", e)),
        _ => {}
      }
      buf.clear();
    }
    Ok(out)
  }

  pub async fn upload_part(&self, bucket: &str, key: &str, upload_id: &str, part_number: i32, data: Vec<u8>) -> Result<String> {
    let encoded_key = encode_object_key(key);
    let path = format!("/{}/{}?partNumber={}&uploadId={}", percent_encode(bucket), encoded_key, part_number, percent_encode(upload_id));
    let mut headers = BTreeMap::new();
    if let Some(kb64) = &self.cfg.sse_c_key_b64 {
      headers.insert("x-amz-server-side-encryption-customer-algorithm".to_string(), "AES256".to_string());
      headers.insert("x-amz-server-side-encryption-customer-key".to_string(), kb64.clone());
      if let Ok(raw) = general_purpose::STANDARD.decode(kb64) {
        let md5sum = md5::compute(&raw);
        headers.insert("x-amz-server-side-encryption-customer-key-md5".to_string(), base64::engine::general_purpose::STANDARD.encode(md5sum.0));
      }
    }
    let resp = self.do_request(Method::PUT, &path, BTreeMap::new(), headers, data).await?;
    let status = resp.status();
    let etag = resp.headers().get("etag").map(|v| v.to_str().unwrap_or_default().to_string()).unwrap_or_default();
    let _ = resp.text().await;
    if !status.is_success() {
      return Err(anyhow!("upload_part error status {}", status));
    }
    Ok(etag)
  }

  pub async fn complete_multipart_upload(&self, bucket: &str, key: &str, upload_id: &str, parts: Vec<CompletedPartResp>) -> Result<serde_json::Value> {
    let encoded_key = encode_object_key(key);
    let path = format!("/{}/{}?uploadId={}", percent_encode(bucket), encoded_key, percent_encode(upload_id));
    let mut xml = String::from("<CompleteMultipartUpload>");
    let mut ps = parts.clone();
    ps.sort_by_key(|p| p.PartNumber);
    for p in ps {
      xml.push_str(&format!("<Part><PartNumber>{}</PartNumber><ETag>{}</ETag></Part>", p.PartNumber, p.ETag));
    }
    xml.push_str("</CompleteMultipartUpload>");
    let resp = self.do_request(Method::POST, &path, BTreeMap::new(), BTreeMap::new(), xml.into_bytes()).await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
      return Err(anyhow!("complete_multipart_upload error {}: {}", status, text));
    }
    Ok(serde_json::json!({ "result": "ok" }))
  }

  pub async fn abort_multipart_upload(&self, bucket: &str, key: &str, upload_id: &str) -> Result<()> {
    let encoded_key = encode_object_key(key);
    let path = format!("/{}/{}?uploadId={}", percent_encode(bucket), encoded_key, percent_encode(upload_id));
    let resp = self.do_request(Method::DELETE, &path, BTreeMap::new(), BTreeMap::new(), Vec::new()).await?;
    let status = resp.status();
    let _ = resp.text().await;
    if !status.is_success() {
      return Err(anyhow!("abort multipart upload error status {}", status));
    }
    Ok(())
  }

  pub async fn download_range(&self, bucket: &str, key: &str, start: Option<u64>, end: Option<u64>) -> Result<Vec<u8>> {
    let encoded_key = encode_object_key(key);
    let path = format!("/{}/{}", percent_encode(bucket), encoded_key);
    let mut headers = BTreeMap::new();
    if start.is_some() {
      let range = if let Some(e) = end { format!("bytes={}-{}", start.unwrap(), e) } else { format!("bytes={}-", start.unwrap()) };
      headers.insert("range".to_string(), range);
    }
    if let Some(kb64) = &self.cfg.sse_c_key_b64 {
      headers.insert("x-amz-server-side-encryption-customer-algorithm".to_string(), "AES256".to_string());
      headers.insert("x-amz-server-side-encryption-customer-key".to_string(), kb64.clone());
      if let Ok(raw) = general_purpose::STANDARD.decode(kb64) {
        let md5sum = md5::compute(&raw);
        headers.insert("x-amz-server-side-encryption-customer-key-md5".to_string(), base64::engine::general_purpose::STANDARD.encode(md5sum.0));
      }
    }
    let resp = self.do_request(Method::GET, &path, BTreeMap::new(), headers, Vec::new()).await?;
    let status = resp.status();
    let bytes = resp.bytes().await?;
    if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
      return Err(anyhow!("download_range error {}: {}", status, String::from_utf8_lossy(&bytes)));
    }
    Ok(bytes.to_vec())
  }

  // append a range to a local temp file (create if not exists)
  pub async fn download_range_append(&self, bucket: &str, key: &str, start: Option<u64>, end: Option<u64>, temp_path: &str) -> Result<u64> {
    let bytes = self.download_range(bucket, key, start, end).await?;
    // ensure directory exists
    if let Some(parent) = std::path::Path::new(temp_path).parent() {
      tokio::fs::create_dir_all(parent).await.ok();
    }
    let mut opt = OpenOptions::new();
    opt.create(true).append(true);
    let mut f = opt.open(temp_path).await?;
    f.write_all(&bytes).await?;
    f.flush().await?;
    Ok(bytes.len() as u64)
  }

  pub fn temp_path_for(&self, key: &str) -> String {
    let proj_dirs = ProjectDirs::from("com", "example", "minio-tauri").unwrap();
    let tmp = proj_dirs.cache_dir();
    let filename = format!("stream_{}_{}", Uuid::new_v4(), sanitize(key));
    let mut path = PathBuf::from(tmp);
    std::fs::create_dir_all(path.as_path()).ok();
    path.push(filename);
    path.to_string_lossy().to_string()
  }

  pub fn clone_for_streaming(&self) -> Self {
    self.clone()
  }

  pub async fn stream_object_to_temp_emit(&self, window: &tauri::Window, bucket: &str, key: &str) -> Result<()> {
    let encoded_key = encode_object_key(key);
    let path = format!("/{}/{}", percent_encode(bucket), encoded_key);
    let mut headers = BTreeMap::new();
    if let Some(kb64) = &self.cfg.sse_c_key_b64 {
      headers.insert("x-amz-server-side-encryption-customer-algorithm".to_string(), "AES256".to_string());
      headers.insert("x-amz-server-side-encryption-customer-key".to_string(), kb64.clone());
      if let Ok(raw) = general_purpose::STANDARD.decode(kb64) {
        let md5sum = md5::compute(&raw);
        headers.insert("x-amz-server-side-encryption-customer-key-md5".to_string(), base64::engine::general_purpose::STANDARD.encode(md5sum.0));
      }
    }
    let url = format!("{}{}", self.base_url, path);
    let url_parsed = reqwest::Url::parse(&url)?;
    let host = match url_parsed.host_str() {
      Some(h) => {
        if let Some(p) = url_parsed.port() {
          format!("{}:{}", h, p)
        } else {
          h.to_string()
        }
      }
      None => return Err(anyhow!("invalid host in url")),
    };
    let signed = self.sign_request(&Method::GET, &host, url_parsed.path(), &BTreeMap::new(), &headers, &[], "s3")?;
    let mut req = self.client.get(url.clone());
    for (k, v) in signed.iter() {
      req = req.header(k, v);
    }
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
      let txt = resp.text().await.unwrap_or_default();
      return Err(anyhow!("stream error {}: {}", status, txt));
    }
    let total_opt = resp.content_length();
    let temp_path = self.temp_path_for(key);
    let mut f = TokioFile::create(&temp_path).await?;
    let mut stream = resp.bytes_stream();
    let mut written: u64 = 0;
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
      let chunk = chunk?;
      f.write_all(&chunk).await?;
      written += chunk.len() as u64;
      let _ = window.emit_all("stream-progress", serde_json::json!({
        "key": key,
        "written": written,
        "total": total_opt
      }));
    }
    f.flush().await?;
    let _ = window.emit_all("stream-complete", serde_json::json!({
      "key": key,
      "path": temp_path
    }));
    Ok(())
  }
}

// -------------------------
// Utilities
// -------------------------
fn percent_encode(s: &str) -> String {
  utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

fn encode_object_key(key: &str) -> String {
  key.split('/').map(|p| percent_encode(p)).collect::<Vec<_>>().join("/")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
  let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
  mac.update(data);
  mac.finalize().into_bytes().to_vec()
}