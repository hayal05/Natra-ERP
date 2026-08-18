use rusqlite::Connection;
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Serialize)]
pub struct BackupInfo { pub path:String, pub size_bytes:u64, pub created_at:String }

pub fn integrity(path:&PathBuf)->Result<String,String>{
    let c=Connection::open(path).map_err(|e|e.to_string())?;
    let result:String=c.query_row("PRAGMA integrity_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    Ok(result)
}

pub fn create(path:&PathBuf, backup_dir:&Path)->Result<BackupInfo,String>{
    if integrity(path)? != "ok" { return Err("Database integrity check failed; backup was not created.".into()); }
    fs::create_dir_all(backup_dir).map_err(|e|e.to_string())?;
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let target=backup_dir.join(format!("natra-erp-{stamp}.sqlite3"));
    let source=Connection::open(path).map_err(|e|e.to_string())?;
    let mut dest=Connection::open(&target).map_err(|e|e.to_string())?;
    {
        let backup=rusqlite::backup::Backup::new(&source,&mut dest).map_err(|e|e.to_string())?;
        backup.run_to_completion(100, std::time::Duration::from_millis(20), None).map_err(|e|e.to_string())?;
    }
    let size=fs::metadata(&target).map_err(|e|e.to_string())?.len();
    Ok(BackupInfo{path:target.to_string_lossy().into_owned(),size_bytes:size,created_at:stamp.to_string()})
}

pub fn list(backup_dir:&Path)->Result<Vec<BackupInfo>,String>{
    if !backup_dir.exists(){return Ok(Vec::new());}
    let mut items=Vec::new();
    for entry in fs::read_dir(backup_dir).map_err(|e|e.to_string())?{
        let entry=entry.map_err(|e|e.to_string())?; let p=entry.path();
        if p.extension().and_then(|x|x.to_str()) != Some("sqlite3"){continue;}
        let meta=fs::metadata(&p).map_err(|e|e.to_string())?;
        items.push(BackupInfo{path:p.to_string_lossy().into_owned(),size_bytes:meta.len(),created_at:meta.modified().ok().and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|d|d.as_secs().to_string()).unwrap_or_default()});
    }
    items.sort_by(|a,b|b.created_at.cmp(&a.created_at)); Ok(items)
}

pub fn prune(backup_dir:&Path, keep:usize)->Result<(),String>{
    let items=list(backup_dir)?;
    for item in items.into_iter().skip(keep){fs::remove_file(item.path).map_err(|e|e.to_string())?;}
    Ok(())
}
