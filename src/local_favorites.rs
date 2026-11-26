//! Android favorite downloads: durable, bounded queue independent of Telegram.
use crate::android_catalog::Track;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json,Value};
use std::{path::Path, sync::Mutex};

pub struct LocalFavorites { db:Mutex<Connection> }
impl LocalFavorites {
    pub fn open(path: &Path)->rusqlite::Result<Self> {Self::from_connection(Connection::open(path)?)}
    fn from_connection(db:Connection)->rusqlite::Result<Self> {
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS favorites (id TEXT PRIMARY KEY, title TEXT NOT NULL, artist TEXT NOT NULL, favorite INTEGER NOT NULL, status TEXT NOT NULL, bytes INTEGER NOT NULL DEFAULT 0, total INTEGER, error TEXT); UPDATE favorites SET status='queued', bytes=0, total=NULL WHERE favorite=1 AND status='downloading';")?;
        Ok(Self{db:Mutex::new(db)})
    }
    pub fn liked(&self)->rusqlite::Result<Vec<Track>> {
        let db=self.db.lock().unwrap();let mut stmt=db.prepare("SELECT id,title,artist FROM favorites WHERE favorite=1 ORDER BY rowid LIMIT 200")?;
        let rows=stmt.query_map([],|r|Ok(Track{id:r.get(0)?,title:r.get(1)?,artist:r.get(2)?,thumbnail:String::new(),url:String::new()}))?;
        rows.collect()
    }
    pub fn like(&self,track:&Track)->rusqlite::Result<()> {
        let db=self.db.lock().unwrap();
        let known:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM favorites WHERE id=?1)",[&track.id],|r|r.get(0))?;
        let count:u64=db.query_row("SELECT COUNT(*) FROM favorites",[],|r|r.get(0))?;
        if !known && count>=2000 {return Err(rusqlite::Error::InvalidQuery);}
        db.execute("INSERT INTO favorites(id,title,artist,favorite,status) VALUES(?1,?2,?3,1,'queued') ON CONFLICT(id) DO UPDATE SET title=excluded.title,artist=excluded.artist,favorite=1,status=CASE WHEN favorites.status='saved' THEN 'saved' ELSE 'queued' END,bytes=0,total=NULL,error=NULL",params![track.id,track.title,track.artist])?; Ok(())
    }
    pub fn unlike(&self,id:&str)->rusqlite::Result<()> {
        self.db.lock().unwrap().execute("UPDATE favorites SET favorite=0,status=CASE WHEN status='saved' THEN 'saved' ELSE 'cancelled' END WHERE id=?1",[id])?;Ok(())
    }
    pub fn active(&self,id:&str)->bool {self.db.lock().unwrap().query_row("SELECT EXISTS(SELECT 1 FROM favorites WHERE id=?1 AND favorite=1 AND status='downloading')",[id],|r|r.get(0)).unwrap_or(false)}
    pub fn next(&self)->rusqlite::Result<Option<Track>> {
        let db=self.db.lock().unwrap();
        let item=db.query_row("SELECT id,title,artist FROM favorites WHERE favorite=1 AND status='queued' ORDER BY rowid LIMIT 1",[],|r|Ok(Track{id:r.get(0)?,title:r.get(1)?,artist:r.get(2)?,thumbnail:String::new(),url:String::new()})).optional()?;
        if let Some(ref t)=item {db.execute("UPDATE favorites SET status='downloading',bytes=0,total=NULL,error=NULL WHERE id=?1 AND favorite=1 AND status='queued'",[&t.id])?;}Ok(item)
    }
    pub fn progress(&self,id:&str,bytes:u64,total:Option<u64>) {let _=self.db.lock().unwrap().execute("UPDATE favorites SET bytes=?2,total=?3 WHERE id=?1 AND favorite=1 AND status='downloading'",params![id,bytes,total]);}
    pub fn finish(&self,id:&str,ok:bool) {let _=self.db.lock().unwrap().execute("UPDATE favorites SET status=CASE WHEN favorite=0 THEN 'cancelled' WHEN ?2=1 THEN 'saved' ELSE 'failed' END,error=CASE WHEN ?2=1 THEN NULL ELSE 'Could not save this song. Check storage permission, space, or source availability; tap Retry.' END WHERE id=?1 AND status IN ('downloading','cancelled')",params![id,ok]);}
    pub fn retry(&self)->rusqlite::Result<usize> {self.db.lock().unwrap().execute("UPDATE favorites SET status='queued',error=NULL,bytes=0,total=NULL WHERE favorite=1 AND status='failed'",[])}
    pub fn status(&self)->rusqlite::Result<Value> {
        let db=self.db.lock().unwrap();let mut stmt=db.prepare("SELECT id,title,status,bytes,total,error FROM favorites WHERE favorite=1 ORDER BY CASE status WHEN 'downloading' THEN 0 WHEN 'queued' THEN 1 WHEN 'failed' THEN 2 ELSE 3 END,rowid LIMIT 200")?;
        let rows=stmt.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"title":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"bytes":r.get::<_,u64>(3)?,"total":r.get::<_,Option<u64>>(4)?,"error":r.get::<_,Option<String>>(5)?})))?;
        Ok(json!({"items":rows.collect::<rusqlite::Result<Vec<_>>>()?}))
    }
}

#[cfg(test)] mod tests {
    use super::*;
    fn track()->Track{Track{id:"abcdefghijk".into(),title:"Song".into(),artist:"Artist".into(),thumbnail:String::new(),url:String::new()}}
    #[test] fn favorites_queue_persists_and_unlike_never_deletes_saved_files() {
        let path=std::env::temp_dir().join(format!("aysha-local-favorites-{}.sqlite",std::process::id()));let _=std::fs::remove_file(&path);
        {let db=LocalFavorites::open(&path).unwrap();db.like(&track()).unwrap();assert_eq!(db.next().unwrap().unwrap().id,"abcdefghijk");assert!(db.active("abcdefghijk"));}
        {let db=LocalFavorites::open(&path).unwrap();assert_eq!(db.liked().unwrap().len(),1);assert!(db.next().unwrap().is_some());db.finish("abcdefghijk",true);db.unlike("abcdefghijk").unwrap();assert_eq!(db.liked().unwrap().len(),0);db.like(&track()).unwrap();assert!(db.next().unwrap().is_none());assert_eq!(db.status().unwrap()["items"][0]["status"],"saved");}
        let _=std::fs::remove_file(path);
    }
    #[test] fn cancelled_transfer_cannot_be_requeued_without_a_new_like() {
        let db=LocalFavorites::from_connection(Connection::open_in_memory().unwrap()).unwrap();db.like(&track()).unwrap();db.next().unwrap();db.unlike("abcdefghijk").unwrap();assert!(!db.active("abcdefghijk"));db.finish("abcdefghijk",false);assert!(db.next().unwrap().is_none());
    }
}
