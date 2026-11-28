//! SQLite-backed user-selected library folder, separate from song data.
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

fn open(db:&Path)->Result<Connection,String> {
    let conn=Connection::open(db).map_err(|_|"local_settings_unavailable".to_owned())?;
    conn.busy_timeout(std::time::Duration::from_secs(2)).map_err(|_|"local_settings_unavailable".to_owned())?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS library_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)").map_err(|_|"local_settings_unavailable".to_owned())?;
    Ok(conn)
}
pub fn load(db:&Path)->Result<Option<String>,String> {
    let conn=open(db)?;
    let value:Option<String>=conn.query_row("SELECT value FROM library_settings WHERE key='folder'",[],|row|row.get(0)).optional().map_err(|_|"local_settings_unavailable".to_owned())?;
    match value {Some(path) if valid(&path)=>Ok(Some(path)),Some(_)=>Err("local_settings_invalid".into()),None=>Ok(None)}
}
fn valid(path:&str)->bool { !path.is_empty() && path.len()<=8192 && !path.contains('\0') && Path::new(path).is_absolute() }
pub fn save(db:&Path,path:&str)->Result<(),String> {
    if !valid(path){return Err("local_settings_invalid".into());}
    let conn=open(db)?;
    conn.execute("INSERT INTO library_settings (key,value) VALUES ('folder',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![path]).map_err(|_|"local_settings_unavailable".to_owned())?;
    Ok(())
}
pub fn forget(db:&Path)->Result<(),String> {
    let conn=open(db)?;
    conn.execute("DELETE FROM library_settings WHERE key='folder'",[]).map_err(|_|"local_settings_unavailable".to_owned())?;
    Ok(())
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn folder_survives_reopen_and_forget_only_removes_setting() {
        let dir=std::env::temp_dir().join(format!("aysha-library-settings-test-{}",std::process::id()));
        std::fs::create_dir(&dir).unwrap();let db=dir.join("settings.sqlite3");let folder=dir.join("music");std::fs::create_dir(&folder).unwrap();
        let song=folder.join("song.m4a");std::fs::write(&song,b"fixture").unwrap();
        assert_eq!(load(&db).unwrap(),None);save(&db,folder.to_str().unwrap()).unwrap();
        assert_eq!(load(&db).unwrap().as_deref(),folder.to_str());
        assert!(save(&db,"relative/path").is_err());assert_eq!(load(&db).unwrap().as_deref(),folder.to_str());
        forget(&db).unwrap();assert_eq!(load(&db).unwrap(),None);assert_eq!(std::fs::read(&song).unwrap(),b"fixture");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
