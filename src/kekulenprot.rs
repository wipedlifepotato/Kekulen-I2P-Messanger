use std::sync::{Arc, Mutex};
use std::{thread, time};
use std::convert::TryInto;

use rand::prelude::*;
use sha2::Sha256;
use hkdf::Hkdf;
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce, aead::Aead};
use libcrux_ml_kem::{mlkem768};

use crate::sam::sam::*;
use crate::config::config::AppConfig;

struct SamSession {
    session: SAM,
    #[allow(dead_key_pair)]
    key_pair: KeyPair,
}

impl SamSession {
    fn is_exists(dat_file: &str) -> bool {
        AppConfig::is_exists(dat_file)
    }

    pub fn import(dat_file: &str, password: &str) -> Self {
        dbg!("import", dat_file);
        if !Self::is_exists(&(dat_file.to_owned() + ".dat")) {
            return Self::new(dat_file, password, 7);
        }
        let mut kp = KeyPair::dummy();
        dbg!(password);
        if !password.is_empty() {
            kp.set_password(password);
        }
        
        kp.load_from_file(dat_file).expect("Can't read dat_file, check your password");
        dbg!(&kp);
        let conf = AppConfig::load();
        let mut sam = SAM::new(conf.host_sam.as_str(), conf.port_sam);
        sam.set_keypair(kp.clone());
        sam.set_nickname(dat_file);
        sam.create_session(dat_file);
        
        dbg!(&sam);
        Self { session: sam, key_pair: kp }            
    }

    pub fn new(dat_file: &str, password: &str, key_type: u8) -> Self {
        dbg!("Create new profile");
        if Self::is_exists(&(dat_file.to_owned() + ".dat")) {
            return Self::import(dat_file, password);
        }
        dbg!("is not exists", &(dat_file.to_owned() + ".dat"));
        let conf = AppConfig::load();
        let mut kp = SAM::new(conf.host_sam.as_str(), conf.port_sam).generate_dest(key_type);
        if !password.is_empty() {
            kp.set_password(password);
        }
        dbg!(&kp);
        let mut sam = SAM::new(conf.host_sam.as_str(), conf.port_sam);
        sam.set_keypair(kp.clone());
        sam.create_session(dat_file);
        sam.set_nickname(dat_file);
        kp.save_to_file(dat_file).expect("Can't save your keypair");
        Self { session: sam, key_pair: kp }
    }
}

#[derive(Clone)]
pub struct Friend {
    pub pub_key: String,
    pub name: String,
    pub key_send: ChaCha20Poly1305,
    pub key_recv: ChaCha20Poly1305,
    pub sam: Arc<Mutex<SAM>>, 
    pub messages: Vec<String>,
    pub send_count: u64,
    pub recv_count: u64,
    pub is_active: bool,
    pub key_exchanged: bool,
}

impl Friend {
    pub fn send_msg(&mut self, message: &str) -> Result<(), String> {
        if !self.key_exchanged {
            return Err("not exchanged key".to_string());
        }
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[0..8].copy_from_slice(&self.send_count.to_le_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        match self.key_send.encrypt(nonce, message.as_bytes()) {
            Ok(ciphertext) => {
                let msg_len = (ciphertext.len() as u32).to_le_bytes();
                
                // Собираем длину и сообщение в один пакет, чтобы избежать фрагментации
                let mut payload = Vec::with_capacity(4 + ciphertext.len());
                payload.extend_from_slice(&msg_len);
                payload.extend_from_slice(&ciphertext);

                let mut sam_lock = self.sam.lock().map_err(|_| "SAM mutex poisoned")?;
                sam_lock.write_bytes(&payload).map_err(|e| e.to_string())?;

                self.messages.push(format!("Me: {}", message));
                self.send_count += 1;
                Ok(())
            }
            Err(e) => Err(format!("Encryption error: {:?}", e)),
        }
    }
}

pub struct Protocol {
    session: Arc<Mutex<SamSession>>,
    friends: Arc<Mutex<Vec<Friend>>>,
}

impl Protocol {
    pub fn create_profile(dat_file: &str, password: &str, key_type: u8) -> Self {
        let s = SamSession::new(dat_file, password, key_type);
        Self { 
            session: Arc::new(Mutex::new(s)), 
            friends: Arc::new(Mutex::new(Vec::new())), 
        }
    }

    pub fn load_profile(dat_file: &str, password: &str) -> Self {
        dbg!("load profile", dat_file);
        let s = SamSession::import(dat_file, password);
        Self { 
            session: Arc::new(Mutex::new(s)), 
            friends: Arc::new(Mutex::new(Vec::new())), 
        }
    }

    pub fn get_my_public_key(&self) -> String {
        self.session.lock().unwrap().key_pair.get_public()
    }

    pub fn connect_thread(&self) {
        let session_ptr = Arc::clone(&self.session);
        let friends_ptr = Arc::clone(&self.friends);
        
        let my_pub_key = self.get_my_public_key();

        dbg!("start connect thread");
        thread::spawn(move || {
            loop {
                let targets: Vec<String> = {
                    let mut f = friends_ptr.lock().unwrap();
                    f.iter_mut()
                        .filter(|f| !f.is_active) 
                        .map(|f| {
                            f.is_active = true; 
                            f.pub_key.clone()
                        })
                        .collect()
                };

                for pub_key in targets {
                    let f_ptr = Arc::clone(&friends_ptr);
                    let s_ptr = Arc::clone(&session_ptr);
                    let my_pk = my_pub_key.clone();

                    thread::spawn(move || {
                        let conf = AppConfig::load();
                        let nickname = s_ptr.lock().unwrap().session.get_nickname();

                        let mut sam = SAM::new(&conf.host_sam, conf.port_sam);
                        sam.set_nickname(&nickname);
                        
                        if sam.connect(&pub_key) {
                            // ИДЕНТИФИКАЦИЯ: Отправляем свой ключ, чтобы Боб мог его считать через read_line_manual
                            let id_packet = format!("{}\n", my_pk);
                            if sam.write_bytes(id_packet.as_bytes()).is_err() {
                                reset_friend_active(&f_ptr, &pub_key);
                                return;
                            }

                            let mut seed = [0u8; 64];
                            thread_rng().fill_bytes(&mut seed);
                            let key_pair = mlkem768::generate_key_pair(seed);

                            if sam.write_bytes(key_pair.public_key().as_slice()).is_err() {
                                reset_friend_active(&f_ptr, &pub_key);
                                return;
                            }

                            let ct_bytes = match sam.read_bytes(1088) {
                                Ok(b) if b.len() == 1088 => b,
                                _ => {
                                    reset_friend_active(&f_ptr, &pub_key);
                                    return;
                                }
                            };
                            let ct_array: [u8; 1088] = ct_bytes.try_into().expect("CT size error");
                            let ct = mlkem768::MlKem768Ciphertext::from(ct_array);

                            let shared_secret = mlkem768::decapsulate(key_pair.private_key(), &ct);
                            
                            let hk = Hkdf::<Sha256>::new(None, shared_secret.as_slice());
                            let mut s_k = [0u8; 32];
                            let mut r_k = [0u8; 32];
                            hk.expand(b"Kekulen-v1-Send-Key", &mut s_k).unwrap();
                            hk.expand(b"Kekulen-v1-Recv-Key", &mut r_k).unwrap();

                            let send_cipher = ChaCha20Poly1305::new(Key::from_slice(&s_k));
                            let recv_cipher = ChaCha20Poly1305::new(Key::from_slice(&r_k));
                            
                            let shared_sam = Arc::new(Mutex::new(sam));

                            {
                                let mut friends = f_ptr.lock().unwrap();
                                if let Some(f) = friends.iter_mut().find(|f| f.pub_key == pub_key) {
                                    f.sam = Arc::clone(&shared_sam);
                                    f.key_recv = recv_cipher.clone();
                                    f.key_send = send_cipher.clone();
                                    f.send_count = 0;
                                    f.is_active = true;
                                    f.key_exchanged = true;
                                }
                            }

                            listen_loop(shared_sam, recv_cipher, f_ptr, pub_key);
                        } else {
                            reset_friend_active(&f_ptr, &pub_key);
                        }
                    });
                }
                thread::sleep(time::Duration::from_secs(30));
            }
        });
    }

    pub fn get_friends_list(&self) -> Arc<Mutex<Vec<Friend>>> {
        Arc::clone(&self.friends)
    }

    pub fn send_to(&self, pub_key: &str, message: &str) -> Result<(), String> {
        let mut friends = self.friends.lock().unwrap();
        if let Some(f) = friends.iter_mut().find(|f| f.pub_key == pub_key) {
            match f.send_msg(message) {
                Err(e) => {
                    f.is_active = false;
                    Err(e)
                },
                Ok(()) => Ok(())
            }
        } else {
            Err("Friend not found".into())
        }
    }

    pub fn accept_thread(&self) {
        let session_ptr = Arc::clone(&self.session);
        let friends_ptr = Arc::clone(&self.friends);

        thread::spawn(move || {
            let conf = AppConfig::load();
            let nickname = session_ptr.lock().unwrap().session.get_nickname();
            dbg!("accept thread", &nickname);
            loop {
                let mut sam = SAM::new(&conf.host_sam, conf.port_sam);
                sam.set_nickname(&nickname);
                dbg!("wait accept");
                
                if sam.accept() {
                    let f_ptr = Arc::clone(&friends_ptr);
                    thread::spawn(move || {
                        let remote_pub_key = read_line_manual(&mut sam);
                        if remote_pub_key.is_empty() { return; }

                        let pk_bytes = match sam.read_bytes(1184) {
                            Ok(b) if b.len() == 1184 => b,
                            _ => return,
                        };
                        
                        let pk = mlkem768::MlKem768PublicKey::try_from(pk_bytes.as_slice()).unwrap();

                        let mut entropy = [0u8; 32];
                        thread_rng().fill_bytes(&mut entropy);
                        let (ct, shared_secret) = mlkem768::encapsulate(&pk, entropy);
                        
                        if sam.write_bytes(ct.as_slice()).is_err() {
                            return;
                        }

                        let hk = Hkdf::<Sha256>::new(None, shared_secret.as_slice());
                        let mut s_k = [0u8; 32];
                        let mut r_k = [0u8; 32];
                        hk.expand(b"Kekulen-v1-Recv-Key", &mut s_k).unwrap();
                        hk.expand(b"Kekulen-v1-Send-Key", &mut r_k).unwrap();

                        let send_cipher = ChaCha20Poly1305::new(Key::from_slice(&s_k));
                        let recv_cipher = ChaCha20Poly1305::new(Key::from_slice(&r_k));

                        let shared_sam = Arc::new(Mutex::new(sam));

                        {
                            let mut friends = f_ptr.lock().unwrap();
                            if let Some(f) = friends.iter_mut().find(|f| f.pub_key == remote_pub_key) {
                                f.sam = Arc::clone(&shared_sam);
                                f.key_send = send_cipher.clone();
                                f.key_recv = recv_cipher.clone();
                                f.send_count = 0;
                                f.is_active = true;
                                f.key_exchanged = true;
                            } else {
                                friends.push(Friend {
                                    pub_key: remote_pub_key.clone(),
                                    name: format!("New Friend ({})", &remote_pub_key[..8]),
                                    key_send: send_cipher.clone(),
                                    key_recv: recv_cipher.clone(),
                                    sam: Arc::clone(&shared_sam),
                                    messages: Vec::new(),
                                    key_exchanged: true,
                                    is_active: true, 
                                    send_count: 0,
                                    recv_count: 0,
                                });
                            }
                        }

                        listen_loop(shared_sam, recv_cipher, f_ptr, remote_pub_key);
                    });
                }
            }
        });
    }
}

fn reset_friend_active(friends_ptr: &Arc<Mutex<Vec<Friend>>>, pub_key: &str) {
    if let Ok(mut friends) = friends_ptr.lock() {
        if let Some(f) = friends.iter_mut().find(|f| f.pub_key == pub_key) {
            f.is_active = false;
        }
    }
}

fn read_line_manual(sam: &mut SAM) -> String {
    let mut buffer = Vec::new();
    while let Ok(b) = sam.read_bytes(1) {
        if b.is_empty() || b[0] == 10 { break; }
        buffer.push(b[0]);
    }
    String::from_utf8_lossy(&buffer).trim().to_string()
}

fn listen_loop(sam: Arc<Mutex<SAM>>, recv_cipher: ChaCha20Poly1305, friends_ptr: Arc<Mutex<Vec<Friend>>>, pub_key: String) {
    let mut nonce_counter: u64 = 0;
    
    loop {
        {
            let friends = friends_ptr.lock().unwrap();
            if let Some(f) = friends.iter().find(|f| f.pub_key == pub_key) {
                if !f.is_active { break; }
            } else { break; }
        }

        let len_result = {
            
            let mut sam_lock = sam.lock().unwrap();
            sam_lock.set_timeout(1);
            sam_lock.read_bytes(4)
        };

        match len_result {
            Ok(len_bytes) if len_bytes.len() == 4 => {
                let msg_len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
                
                let enc_result = {
                    let mut sam_lock = sam.lock().unwrap();
                    sam_lock.read_bytes(msg_len)
                };

                match enc_result {
                    Ok(encrypted_data) => {
                        let mut nonce_bytes = [0u8; 12];
                        nonce_bytes[0..8].copy_from_slice(&nonce_counter.to_le_bytes());
                        let nonce = Nonce::from_slice(&nonce_bytes);

                        match recv_cipher.decrypt(nonce, encrypted_data.as_ref()) {
                            Ok(plaintext) => {
                                if let Ok(msg) = String::from_utf8(plaintext) {
                                    let mut friends = friends_ptr.lock().unwrap();
                                    if let Some(f) = friends.iter_mut().find(|f| f.pub_key == pub_key) {
                                        f.messages.push(msg);
                                        f.recv_count += 1;
                                    }
                                }
                                nonce_counter += 1;
                            }
                            Err(_) => break,
                        }
                    }
                    Err(_) => break,
                }
            }
            Ok(_) => {
                thread::sleep(time::Duration::from_millis(50));
            }
            Err(_) => {
                thread::sleep(time::Duration::from_millis(50));
            }
        }
    }
    reset_friend_active(&friends_ptr, &pub_key);
}