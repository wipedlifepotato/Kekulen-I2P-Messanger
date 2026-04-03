mod sam;
mod tui;
mod config;
mod kekulenprot;

#[allow(unused_imports)]
use sam::sam::{SAM,KeyPair};
use kekulenprot::*;
fn main() {
//    key_pair.save_to_file("test");
//    let p = key_pair.get_public();
//    assert_eq!(p,key_pair.load_from_file("test").expect("Can't load from file").get_public());
//    dbg!("key is ok");
//    return;
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
