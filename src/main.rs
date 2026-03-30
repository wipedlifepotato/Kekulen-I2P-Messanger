mod sam;
mod tui;
use mid;
#[allow(unused_imports)]
use sam::sam::{SAM,KeyPair};
use argon2::Argon2;

fn main() {
    let mut key_pair = SAM::new("127.0.0.1", 7656).generate_dest(7) ;
    let password = "super_password";
    let mid_data = mid::get(password).unwrap();
    let mut output_key_material = [0u8; 32];
    let mut output_salt_material = [0u8; 12];
    Argon2::default()
    .hash_password_into(password.as_bytes(), mid_data.as_bytes(), &mut output_key_material).unwrap();
    Argon2::default()
    .hash_password_into(password.as_bytes(), mid_data.as_bytes(), &mut output_salt_material).unwrap();
    key_pair.set_key(output_key_material, output_salt_material);
    key_pair.save_to_file("test");
    let p = key_pair.get_public();
    assert_eq!(p,key_pair.load_from_file("test").expect("Can't load from file").get_public());
    dbg!("key is ok");
    return;
  //  tui::run();
    /*
     /* ACCEPT EXAMPLE EC*HO BOT */
     let mut m = SAM::new("127.0.0.1", 7656);
     let key_pair = SAM::new("127.0.0.1", 7656).generate_dest(7) ;
     dbg!(&key_pair);
     m.set_keypair( key_pair );
     m.create_session("test2");
     loop {
     dbg!("Accept");
     let mut accept = SAM::new("127.0.0.1", 7656);
     accept.set_nickname("test2");
     accept.accept();
     dbg!("read data");
     let incoming = accept.read_str();
     println!("New incoming connection with {}", incoming);
     while accept.isactive() {
         accept.write_str("HELLO ECHO");
         let ndata = accept.read_str();
         accept.write_string(ndata);
}


}
//let mut m1 = SAM::new("127.0.0.1", 7656);
//m1.set_nickname("test");
//m1.connect("");
*/
     let mut m = SAM::new("127.0.0.1", 7656);
     m.create_session("test");
     let mut m1 = SAM::new("127.0.0.1", 7656);
     m1.set_nickname("test");
     if ! m1.connect("UeAnMVIMYaq2-uwy6dlZuoXYL4CZKKakwFPY~EDX5LNR4CcxUgxhqrb67DLp2Vm6hdgvgJkopqTAU9j8QNfks1HgJzFSDGGqtvrsMunZWbqF2C-AmSimpMBT2PxA1-SzUeAnMVIMYaq2-uwy6dlZuoXYL4CZKKakwFPY~EDX5LNR4CcxUgxhqrb67DLp2Vm6hdgvgJkopqTAU9j8QNfks1HgJzFSDGGqtvrsMunZWbqF2C-AmSimpMBT2PxA1-SzUeAnMVIMYaq2-uwy6dlZuoXYL4CZKKakwFPY~EDX5LNR4CcxUgxhqrb67DLp2Vm6hdgvgJkopqTAU9j8QNfks1HgJzFSDGGqtvrsMunZWbqF2C-AmSimpMBT2PxA1-SzUeAnMVIMYaq2-uwy6dlZuoXYL4CZKKakwFPY~EDX5LNR4CcxUgxhqrb67DLp2Vm6hdgvgJkopqTAU9j8QNfks-VxQAoODnyWOrGG-BrnE2YgoqkoYJK5IFFUfUN0UcboBQAEAAcAAA==")
     {
         todo!("can't connect");
     }
     m1.write_string("GET / HTTP/1.1\r\n\r\n\r\n".to_string());
     dbg!(m1.read_str());
}
