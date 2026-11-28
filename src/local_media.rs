//! Seekable file-backed audio. Rust owns the descriptor and closes it on drop.
use crate::range_media::{Block,Fetch,BLOCK};
use std::{fs::File,io};
#[cfg(unix)] use std::os::unix::fs::FileExt;
#[cfg(windows)] use std::os::windows::fs::FileExt;
const MAX_FILE:u64=64*1024*1024;

pub struct LocalFile {file:File,len:u64}
impl LocalFile {
    pub fn new(file:File)->io::Result<Self> {
        let len=file.metadata()?.len();
        if len<12 || len>MAX_FILE {return Err(io::Error::other("local audio size invalid"));}
        let mut header=[0u8;12];read_exact_at(&file,&mut header,0)?;
        if &header[4..8]!=b"ftyp" {return Err(io::Error::other("local audio is not MP4"));}
        Ok(Self{file,len})
    }
    pub fn len(&self)->u64{self.len}
}
fn read_exact_at(file:&File,bytes:&mut [u8],offset:u64)->io::Result<()> {
    let mut read=0;
    while read<bytes.len() {
        #[cfg(unix)] let result=file.read_at(&mut bytes[read..],offset+read as u64);
        #[cfg(windows)] let result=file.seek_read(&mut bytes[read..],offset+read as u64);
        match result {
            Ok(0)=>return Err(io::Error::new(io::ErrorKind::UnexpectedEof,"local audio truncated")),
            Ok(n)=>read+=n,
            Err(e) if e.kind()==io::ErrorKind::Interrupted=>continue,
            Err(e)=>return Err(e),
        }
    }
    Ok(())
}
impl Fetch for LocalFile {
    fn fetch(&self,start:u64,end:u64)->io::Result<Block> {
        if start>end || start>=self.len || end-start>=BLOCK as u64 {return Err(io::Error::other("local range invalid"));}
        let end=end.min(self.len-1);let mut bytes=vec![0u8;(end-start+1) as usize];
        read_exact_at(&self.file,&mut bytes,start)?;
        Ok(Block{bytes,total:self.len})
    }
}

#[cfg(windows)]
pub struct MemoryFile {bytes:Vec<u8>}
#[cfg(windows)]
impl MemoryFile {pub fn new(bytes:Vec<u8>)->Self {Self{bytes}}}
#[cfg(windows)]
impl Fetch for MemoryFile {
    fn fetch(&self,start:u64,end:u64)->io::Result<Block> {
        let len=self.bytes.len() as u64;
        if start>end || start>=len || end-start>=BLOCK as u64 {return Err(io::Error::other("memory range invalid"));}
        let end=end.min(len-1);
        Ok(Block{bytes:self.bytes[start as usize..=end as usize].to_vec(),total:len})
    }
}

#[cfg(test)] mod tests {
    use super::*;use std::io::Write;
    #[test] fn local_ranges_are_bounded_and_detect_truncation() {
        let path=std::env::temp_dir().join(format!("aysha-local-audio-{}.m4a",std::process::id()));
        let mut file=File::create(&path).unwrap();let mut bytes=vec![0u8;3*BLOCK+17];bytes[4..8].copy_from_slice(b"ftyp");for (i,b) in bytes.iter_mut().enumerate().skip(12){*b=(i%251)as u8;}file.write_all(&bytes).unwrap();drop(file);
        let local=LocalFile::new(File::open(&path).unwrap()).unwrap();let prepared=crate::range_media::PreparedFetch::new(Box::new(local),Some(bytes.len() as u64)).unwrap();
        assert_eq!(prepared.fetch(0,31).unwrap().bytes,&bytes[..32]);
        assert_eq!(prepared.fetch((BLOCK-2)as u64,(BLOCK+16)as u64).unwrap().bytes,&bytes[BLOCK-2..BLOCK+17]);
        assert!(prepared.fetch(0,BLOCK as u64).is_err());
        std::fs::OpenOptions::new().write(true).open(&path).unwrap().set_len(12).unwrap();
        // A request that cannot be satisfied from prepared head/tail must fail.
        assert!(prepared.fetch((BLOCK+100) as u64,(BLOCK+200) as u64).is_err());
        let _=std::fs::remove_file(path);
    }
    #[test] fn rejects_invalid_or_short_files() {
        let path=std::env::temp_dir().join(format!("aysha-invalid-audio-{}.m4a",std::process::id()));
        std::fs::write(&path,b"not audio data").unwrap();assert!(LocalFile::new(File::open(&path).unwrap()).is_err());
        let _=std::fs::remove_file(path);
    }
}
