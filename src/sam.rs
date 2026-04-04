pub mod sam {
use regex::Regex;
use std::io::prelude::*;
use std::net::Shutdown;
use std::io;

use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use directories::ProjectDirs;
use std::path::PathBuf;

use chacha20::ChaCha20;
// Import relevant traits
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
//use hex_literal::hex;

#[derive(Debug)]
pub struct SAM{
    is_active: bool,
    t_stream: std::net::TcpStream,
    version: f32,
    key_pair: KeyPair,
    is_master: bool,
    nickname: String,
}

//const DEFAULT_TIMEOUT_SOCKET: std::time::Duration =
//    std::time::Duration::new(30,0);
const HANDSHAKE_MESSAGE: &str = "HELLO VERSION MIN=3.0\n";

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct KeyPair {
    #[serde(rename = "public")]
    public: String,
    #[serde(rename = "private")]
    private: String,
    key: [u8; 32],
    iv: [u8; 12],
}
#[cfg(not(target_os = "android"))]
use mid;
use argon2::Argon2;
impl KeyPair {
    pub fn new(public: String, private: String) -> Self {
        Self{public: public, private: private, ..Default::default()}
    }
    pub fn dummy() -> Self {
        Self { ..Default::default() }
    }
    pub fn set_key(&mut self, key: [u8; 32], iv: [u8; 12]) {
        self.iv = iv.clone();
        self.key = key.clone();
    }
    pub fn get_app_dir() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "wipedlifepotato", "Kekulen") {
            let config_dir = proj_dirs.config_dir();
            fs::create_dir_all(config_dir).expect("can't create directory");
            return config_dir.to_path_buf();
        }
        PathBuf::from(".")
    }

    pub fn get_public(&self) -> String {
        self.public.clone()
    }
    pub fn set_password(&mut self, password: &str) {
                #[cfg(not(target_os = "android"))]
                let mid_data = mid::get(password).unwrap_or_else(|_| {
                    let user = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default();
                    let host = hostname::get().map(|h| h.to_string_lossy().into_owned()).unwrap_or_default();
                    format!("{}-{}", user, host)
                });

                #[cfg(target_os = "android")]
                let mid_data = format!("android-{}", password);

                let mut output_key_material = [0u8; 32];
                let mut output_salt_material = [0u8; 12];
                Argon2::default()
                .hash_password_into(password.as_bytes(), mid_data.as_bytes(), &mut output_key_material).unwrap();
                Argon2::default()
                .hash_password_into(password.as_bytes(), mid_data.as_bytes(), &mut output_salt_material).unwrap();
                self.set_key(output_key_material, output_salt_material);
    }
    pub fn save_to_file(&self, name: &str) -> std::io::Result<()> {
        dbg!("save_to_file");
        let filename = format!("{}.dat", name);
        let file_path = Self::get_app_dir().join(filename);
        dbg!("save", &file_path);
        let serialized = serde_json::to_string(self).expect("Serialize error");
        dbg!(&serialized);
        if self.iv[0] != 0 && self.key[0] != 0{
            let mut cipher = ChaCha20::new(&self.key.into(), &self.iv.into());
            let mut buffer = serialized.into_bytes();
            cipher.apply_keystream(&mut buffer);
            return fs::write(file_path, buffer);
        }
        else {
            fs::write(file_path, serialized)
        }
    }

    pub fn load_from_file(&mut self, name: &str) -> Option<Self> {
        let filename = format!("{}.dat", name);
        let file_path = Self::get_app_dir().join(filename) ;//filename;
        dbg!("load from file");
        if file_path.exists() {
            let data = fs::read(file_path).ok()?;
            dbg!(&data);
            if self.iv[0] != 0 && self.key[0] != 0 {
                let mut cipher = ChaCha20::new(&self.key.into(), &self.iv.into());
                let mut buffer = data;
                cipher.seek(0u32);
                cipher.apply_keystream(&mut buffer);
                if let Ok(json_str) = String::from_utf8(buffer) {
                    dbg!(&json_str);
                         match serde_json::from_str::<KeyPair>(&json_str) {
                        Ok(kp) => {
                            self.public = kp.public.clone();
                            self.private = kp.private.clone();
                            return Some(kp);
                        },
                        Err(e) => {
                            eprintln!("!!! SERDE В SAM.RS: {}", e);
                            eprintln!("line: {}, column: {}", e.line(), e.column());
                            return None;
                        }
                    }
                }
                dbg!("can't decrypt");
                return None;
            }    
            
            serde_json::from_slice(&data).ok()
        } else {
            dbg!("File not exists");
            None
        }
    }
}

impl SAM {
    pub fn isactive(self: &mut Self) -> bool {
        self.is_active
    }
    pub fn write_str(self: &mut Self, msg: &str) -> Result<(), io::Error> {
        return self.write(msg.to_string());
    }
    pub fn write_string(self: &mut Self, msg: String) -> Result<(), io::Error> {
        return self.write(msg);
    }
    pub fn write_bytes<T: AsRef<[u8]>>(&mut self, msg: T) -> Result<(), io::Error> {
        let bytes = msg.as_ref();
        let expected_len = bytes.len();

        match self.t_stream.write_all(bytes) {
            Err(e) => {
                self.is_active = false;
                eprintln!("write error: {:?}", e);
                Err(e)
            },
            Ok(_) => {
                self.t_stream.flush()?; 
                Ok(())
            }
        }
    }
    pub fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; len];

        match self.t_stream.read_exact(&mut buf) {
            Ok(_) => Ok(buf),
            Err(e) => {
                self.is_active = false;
                eprintln!("read error (expected {} bytes): {:?}", len, e);
                Err(e)
            }
        }
    }
    fn write(self: & mut Self, msg: String) -> Result<(), io::Error> {
        let counter = msg.len();
        match self.t_stream.write(msg.as_bytes()) {
            Err(e) => {
                self.is_active = false;
                eprintln!("write error: {:?}", e);
                return Err(e);
            },
            Ok(x) => {
                if x != counter {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken connection"));
                }
            }
        }
        Ok(())
    }
    pub fn set_keypair(self: &mut Self, k: KeyPair) {
        dbg!("set key_pair", &k);
        self.key_pair = k;
    }
    // TODO: test
    pub fn close_session(self: &mut Self, raw_nickname: &str) {
        let nickname = Path::new(raw_nickname)
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("");

        self.write(format!("SESSION REMOVE ID={}\n", nickname)).expect("cant close session");
        //let mut buffer = ;
        let _= self.t_stream.read(&mut [0u8; 2056]);
    }
    pub fn create_session(self: &mut Self, raw_nickname: &str) -> bool {
        let nickname = Path::new(raw_nickname)
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("");
        if self.is_master {
            eprintln!("before created session");
            return false;
        }
        let mut privkey: String;
        if self.key_pair.private.is_empty() {
            // TRANSIENT
            privkey = "TRANSIENT".to_string();
        } else {
            privkey = self.key_pair.private.clone();
        }
        dbg!(&self.key_pair);
        dbg!(&privkey);
        self.write(format!("SESSION CREATE STYLE=STREAM ID={} DESTINATION={}\n", nickname, privkey)).expect("cant create session");
        let mut buffer = [0u8; 2056];
        let _= self.t_stream.read(&mut buffer);
        let line = String::from_utf8_lossy(&buffer);
        let re = Regex::new(r"SESSION STATUS RESULT=(\w+) DESTINATION=.+").expect("cant compile regex");
        dbg!(&line);
        let caps = re.captures(&line).unwrap();
        if caps.len() == 1 {
            return false;
        }
        if caps[1] == *"OK" {
            return true;
        }
        self.is_master = true;
        self.nickname = String::from(nickname);
        dbg!(&caps[0]);
        return false;
    }
    pub fn get_nickname(&self) -> String {
        self.nickname.clone()
    }
    pub fn connect(self: &mut Self, destination: &str) -> bool{
        if self.is_master {
            eprintln!("is master socket");
            return false;
        }
        if self.nickname.is_empty() {
            eprintln!("Need nickname for session");
            return false;
        }
        self.write( format!("STREAM CONNECT ID={} DESTINATION={}\n", self.nickname, destination) ).expect("Can't connect to destination");
        let mut buffer = [0u8; 2056];
        let _= self.t_stream.read(&mut buffer);
        let line = String::from_utf8_lossy(&buffer);
        println!("{}", line);
        return true;
    }
    pub fn accept(self: &mut Self) -> bool {
        self.write( format!("STREAM ACCEPT ID={}\n", self.nickname) ).expect("Can't connect to destination");
        let mut buffer1 = [0u8; 25];
        let _= self.t_stream.read(&mut buffer1);
        let line = String::from_utf8_lossy(&buffer1);
        dbg!(&line);
        if "STREAM STATUS RESULT=OK\n\0" != line {
            return false;
        }
        let mut buffer = [0u8; 2056];
        let _= self.t_stream.read(&mut buffer);
        let accept_line = String::from_utf8_lossy(&buffer);
        dbg!("accept line", accept_line);
        return true;
    }
    const DEF_READ_BUF_SIZE:usize = 1024;
    pub fn read_str(self: &mut Self) -> String {
        let mut buffer = [0u8; Self::DEF_READ_BUF_SIZE];
        let count = self.t_stream.read(&mut buffer);
        //if count != size {
        //    todo!();
        //}
        match count {
            Ok(x) => {
               if x == 0 {
                   self.is_active = false;
               }
            },
            Err(_) => {
                self.is_active = false;
            }
        }
        return String::from_utf8_lossy(&buffer).to_string();
    }
    pub fn set_timeout(&mut self, seconds: u64) -> Result<(), Box<dyn std::error::Error>> {
        if seconds == 0 {
            self.t_stream.set_read_timeout(None)?;
        } else {
            self.t_stream.set_read_timeout(Some(std::time::Duration::from_secs(seconds)))?;
        }
        Ok(())
    }
    pub fn set_nickname(self: &mut Self, nick: &str) {
        self.nickname = String::from(nick);
    }
    pub fn generate_dest(self: &mut Self,v: u8) -> KeyPair {
        self.write( format!("DEST GENERATE SIGNATURE_TYPE={}\n", v) ).expect("Can't generate address [write]");
        let mut buffer = [0u8; 2056];
        let _= self.t_stream.read(&mut buffer);
        let line = String::from_utf8_lossy(&buffer);
        //line.to_string()

        let re = Regex::new(r"DEST REPLY PUB=([\w\-~=]+) PRIV=([\w\-~=]+)").expect("can;'t compile regex");
        let data = re.
        captures_iter(&line).map(|caps| {
            let public = caps.get(1).map(|q| q.as_str()).unwrap_or("-");
            let private = caps.get(2).map(|q| q.as_str()).unwrap_or("-");
            if public != "-" && private != "-" {
                return Ok ( (public,private) );
            }
            Err("can't get version")
        }).next();
       // dbg!(&data);
        match data {
            Some(Ok(x)) => {
                let (public, private) = x;
                return KeyPair{public: public.to_string(), 
                    private: private.to_string(), ..Default::default()};
            },
            Some(Err(_)) => {
                todo!("Can't get destgen data")
            },
            None => {
                todo!();
            }
        }
    }
    pub fn get_version(self: & Self) -> f32 {
        return self.version;
    }
    pub fn close_stream(self: &mut Self) {
        self.is_active = false;
        self.t_stream.shutdown(Shutdown::Both).expect("shutdown call failed");
    }
    pub fn new(host: &str, port: u16) -> Self {
        let h = format!("{}:{}", host,port);
        let mut stream = match std::net::
            TcpStream::connect(h)
         {
            Ok(x) => {
                x
            }, 
            Err(_) => {
                todo!("Can't connect to SAM logic");
                // or expect just
            }
        };

        if stream.write(HANDSHAKE_MESSAGE.as_bytes()).expect("Can't write to socket") != HANDSHAKE_MESSAGE.len() {
            todo!("is not active socket for a now");
        }
        let mut buffer = [0u8; 512];
        let _= stream.read(&mut buffer);
        let line = String::from_utf8_lossy(&buffer);
        //dbg!(&line);
        let r = Regex::new(r"HELLO REPLY RESULT=(\w+) VERSION=(\d+\.\d+)").unwrap();
        let data = r.
            captures_iter(&line).map(|caps| {
                let res = caps.get(1).map(|q| q.as_str()).unwrap_or("-");
                let version = caps.get(2).map(|q| q.as_str()).unwrap_or("-");
                if version != "-" && res != "-" {
                    return Ok ( (version,res) );
                }
                Err("can't get version")
        }).next();
        //dbg!(&data);
        let mut _v:f32 = 0.0;
        let mut _result;
        match data {
         Some(Ok(x)) => {
            let (_version, _status) = x;
          //  dbg!(_version);
            _v = _version.parse().unwrap();
            _result = String::from(_status);
          }, 
         Some(Err(_)) => {
             todo!("Can't connect to sam")
            },
         None => {
                todo!();
                }
        }
        if _result != "OK" {
            todo!("can't create session logic");
        }
        Self {is_active: true, t_stream: stream, version: _v, key_pair: KeyPair{private:String::new(),public:String::new(), ..Default::default()},is_master: false, nickname: String::new()}
    }
}
}
