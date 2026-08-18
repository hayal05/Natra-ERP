use rusqlite::Connection;
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Serialize)]
pub struct BackupInfo { pub path:String, pub size_bytes:u64, pub created_at:String }

pub fn integrity(path:&PathBuf)->Result<String,String>{
    let c=Connection::open(path).map_err(|e|e.to_string())?;
    c.query_row("PRAGMA integrity_check",[],|r|r.get(0)).map_err(|e|e.to_string())
}

pub fn create(path:&PathBuf, backup_dir:&Path)->Result<BackupInfo,String>{
    if integrity(path)? != "ok" { return Err("Database integrity check failed; backup was not created.".into()); }
    fs::create_dir_all(backup_dir).map_err(|e|e.to_string())?;
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let target=backup_dir.join(format!("natra-erp-{stamp}.sqlite3"));
    let source=Connection::open(path).map_err(|e|e.to_string())?;
    let mut dest=Connection::open(&target).map_err(|e|e.to_string())?;
    { let backup=rusqlite::backup::Backup::new(&source,&mut dest).map_err(|e|e.to_string())?; backup.run_to_completion(100,std::time::Duration::from_millis(20),None).map_err(|e|e.to_string())?; }
    let size=fs::metadata(&target).map_err(|e|e.to_string())?.len();
    Ok(BackupInfo{path:target.to_string_lossy().into_owned(),size_bytes:size,created_at:stamp.to_string()})
}

pub fn list(backup_dir:&Path)->Result<Vec<BackupInfo>,String>{
    if !backup_dir.exists(){return Ok(Vec::new());}
    let mut items=Vec::new();
    for entry in fs::read_dir(backup_dir).map_err(|e|e.to_string())?{ let p=entry.map_err(|e|e.to_string())?.path(); if p.extension().and_then(|x|x.to_str())!=Some("sqlite3"){continue;} let meta=fs::metadata(&p).map_err(|e|e.to_string())?; items.push(BackupInfo{path:p.to_string_lossy().into_owned(),size_bytes:meta.len(),created_at:meta.modified().ok().and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|d|d.as_secs().to_string()).unwrap_or_default()}); }
    items.sort_by(|a,b|b.created_at.cmp(&a.created_at)); Ok(items)
}

pub fn prune(backup_dir:&Path,keep:usize)->Result<(),String>{ for item in list(backup_dir)?.into_iter().skip(keep){fs::remove_file(item.path).map_err(|e|e.to_string())?;} Ok(()) }

pub fn restore(path:&PathBuf, backup_path:&Path)->Result<BackupInfo,String>{
    if !backup_path.exists(){return Err("Selected backup does not exist.".into());}
    if backup_path.extension().and_then(|x|x.to_str())!=Some("sqlite3"){return Err("Invalid backup file.".into());}
    if integrity(&backup_path.to_path_buf())? != "ok" {return Err("Selected backup failed integrity check.".into());}
    let backup_dir=path.parent().unwrap_or(Path::new(".")).join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e|e.to_string())?;
    let safety=create(path, &backup_dir)?;
    let temp=path.with_extension("restore.tmp.sqlite3");
    let _=fs::remove_file(&temp);
    fs::copy(backup_path,&temp).map_err(|e|e.to_string())?;
    if integrity(&temp)? != "ok" {let _=fs::remove_file(&temp);return Err("Restored database failed integrity check.".into());}
    let wal=path.with_extension("sqlite3-wal"); let shm=path.with_extension("sqlite3-shm");
    let old=path.with_extension("pre-restore.sqlite3"); let _=fs::remove_file(&old);
    fs::rename(path,&old).map_err(|e|e.to_string())?;
    if let Err(e)=fs::rename(&temp,path){let _=fs::rename(&old,path);return Err(e.to_string());}
    let _=fs::remove_file(&wal); let _=fs::remove_file(&shm);
    Ok(safety)
}
