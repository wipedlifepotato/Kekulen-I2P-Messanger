pub mod kekulenprot {

    #[allow(unused_imports)]
    use crate::sam::sam::*;
    use crate::config::config::AppConfig;

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
            
            let conf = AppConfig::new("config.yaml");
            let mut sam = SAM::new(conf.host_sam.as_str(), conf.port_sam);
            sam.set_keypair(kp.clone());
            sam.create_session(dat_file);
            Self { session: sam, keyPair: kp.clone() }            
        }
        pub fn new(dat_file: &str, password: &str, key_type: u8) -> Self {
            if Self::is_exists(dat_file) {
                return Self::import(dat_file, password);
            }
            let conf = AppConfig::new("config.yaml");
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
    struct Friend {
        pub_key: String,
        name: String,
 //       x2556_key: String,
    }
    impl Friend {
        fn new(pub_key: &str, name: &str, x2556_key: &str){
            todo!("");
        }
    }
}