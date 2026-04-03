pub mod kekulenprot {
    use std::time;

    #[allow(unused_imports)]
    use crate::sam::sam::*;
    use crate::config::config::AppConfig;
    use std::thread;

    struct SamSession {
        session: SAM,
        keyPair: KeyPair,
    }
    impl SamSession {
        fn is_exists(dat_file: &str) -> bool {
            AppConfig::is_exists(dat_file)
        }
        pub fn import(dat_file: &str, password: &str) -> Self {
            if !Self::is_exists(dat_file) {
                return Self::new(dat_file, password, 7);
            }
            let mut kp = KeyPair::dummy();
            if password.len() > 0 {
                kp.set_password(password);
            }
            kp.load_from_file(dat_file).expect("Can't read dat_file, check your password");
            
            let conf = AppConfig::load();
            let mut sam = SAM::new(conf.host_sam.as_str(), conf.port_sam);
            sam.set_keypair(kp.clone());
            sam.create_session(dat_file);
            Self { session: sam, keyPair: kp.clone() }            
        }
        pub fn new(dat_file: &str, password: &str, key_type: u8) -> Self {
            if Self::is_exists(dat_file) {
                return Self::import(dat_file, password);
            }
            let conf = AppConfig::load();
            let mut kp = SAM::new(conf.host_sam.as_str(), conf.port_sam).generate_dest(key_type) ;
            if password.len() > 0 {
                kp.set_password(password);
            }
            let mut sam = SAM::new(conf.host_sam.as_str(), conf.port_sam);
            sam.create_session(dat_file);
            kp.save_to_file(dat_file).expect("Can't save your keypair, check permissions");
            Self{ session: sam, keyPair: kp  }
        }
    }
    #[derive(Clone)]
    struct Friend {
        pub_key: String,
        name: String,
 
    }
    use std::sync::{Arc, Mutex};
    pub struct Protocol {
        session: Arc<Mutex<SamSession>>,
        friends: Arc<Mutex<Vec<Friend>>>,
    }

    impl Protocol {
        pub fn create_profile(dat_file: &str, password: &str, key_type: u8) -> Self {
            if SamSession::is_exists(dat_file) {
                panic!("exist dat_file");   
            }
            let mut s = SamSession::new(dat_file, password, key_type);
            Self { session: Arc::new(Mutex::new(s)), friends: Arc::new(Mutex::new(Vec::new())), }
        }

        pub fn load_profile(dat_file: &str, password: &str) -> Self {
            if !SamSession::is_exists(dat_file) { 
                panic!("Not exist dat_file");
            }
            let mut s = SamSession::import(dat_file, password);
            Self { session: Arc::new(Mutex::new(s)), friends: Arc::new(Mutex::new(Vec::new())), }
        }
        pub fn connect_thread(&self) {
            let session_ptr = Arc::clone(&self.session);
            let friends_ptr = Arc::clone(&self.friends);

            thread::spawn(move || {
                loop {
                    let friends_list = {
                        friends_ptr.lock().expect("Friends mutex poisoned").clone()
                    };

                    for f in friends_list {
                        let s_ptr = Arc::clone(&session_ptr);
                        
                        let pub_key = f.pub_key.clone(); 

                        thread::spawn(move || {
                            let conf = AppConfig::load();
                            
                            let nickname = {
                                let s = s_ptr.lock().expect("Session mutex poisoned");
                                s.session.get_nickname() 
                            };

                            let mut sam = SAM::new(&conf.host_sam, conf.port_sam);
                            sam.set_nickname(&nickname);
                            
                            if sam.connect(&pub_key) {
                                todo!("logic");
                            }
                        });
                    }

                    thread::sleep(std::time::Duration::from_secs(20));
                }
            });
        }
        fn accept_hread(&mut self) {
            let session_ptr = Arc::clone(&self.session);
            thread::spawn(move || {
                loop {
                    let conf = AppConfig::load();

                    let nickname = {
                        let s = session_ptr.lock().unwrap();
                        s.session.get_nickname()
                    };
                    let mut sam = SAM::new(conf.host_sam.as_str(), conf.port_sam);
                    sam.set_nickname(&nickname);
                    sam.accept();
                    let incoming = sam.read_str();
                    dbg!(incoming);
                    todo!("logic");
                    thread::spawn(move || {
                        while sam.isactive() {
                            let ndata = sam.read_str();
                        }
                    });
                }            
            });
        }
    }

    impl Friend {
        fn new(pub_key: &str, name: &str){
            todo!("");
        }
    }
}