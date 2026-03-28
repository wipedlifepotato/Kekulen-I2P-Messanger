pub mod sam {
use regex::Regex;
use std::io::prelude::*;
use std::net::Shutdown;
use std::io;
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

#[derive(Debug, PartialEq)]
pub struct KeyPair {
    public: String,
    private: String,
}
impl KeyPair {
    pub fn get_public(self: &Self) -> String {
        self.public.clone()
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
        self.key_pair = k;
    }
    // TODO: test
    pub fn close_session(self: &mut Self, nickname: &str) {
        self.write(format!("SESSION REMOVE ID={}\n", nickname)).expect("cant close session");
        //let mut buffer = ;
        let _= self.t_stream.read(&mut [0u8; 2056]);
    }
    pub fn create_session(self: &mut Self, nickname: &str) -> bool {
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
        self.write(format!("SESSION CREATE STYLE=STREAM ID={} DESTINATION={}\n", nickname, privkey)).expect("cant create session");
        let mut buffer = [0u8; 2056];
        let _= self.t_stream.read(&mut buffer);
        let line = String::from_utf8_lossy(&buffer);
        let re = Regex::new(r"SESSION STATUS RESULT=(\w+) DESTINATION=.+").expect("cant compile regex");
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
        if "STREAM STATUS RESULT=OK" != line {
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
                return KeyPair{public: public.to_string(), private: private.to_string()};
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
   //     let _= stream.
   //         set_write_timeout(Some(DEFAULT_TIMEOUT_SOCKET));
   //     let _= stream.set_read_timeout(Some(DEFAULT_TIMEOUT_SOCKET));

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
        Self {is_active: true, t_stream: stream, version: _v, key_pair: KeyPair{private:String::new(),public:String::new()},is_master: false, nickname: String::new()}
    }
}
}
