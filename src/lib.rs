use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use std::sync::{Arc, OnceLock};

mod api;
pub mod config;
pub mod kekulenprot;
pub mod sam;

use crate::api::*;
use crate::kekulenprot::Protocol;

static PROTOCOL: OnceLock<Arc<Protocol>> = OnceLock::new();

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_kekulen_app_ProtocolBridge_initProtocol(
    mut env: JNIEnv,
    _class: JClass,
    dat_file: JString,
    pass: JString,
) {
    let dat: String = env.get_string(&dat_file).expect("Bad dat path").into();
    let password: String = env.get_string(&pass).expect("Bad password").into();

    let proto = Protocol::load_profile(&dat, &password);
    let proto_arc: Arc<Protocol> = Arc::new(proto);

    let p_clone = Arc::clone(&proto_arc);
    std::thread::spawn(move || {
        p_clone.accept_thread();
    });

    let p_clone2 = Arc::clone(&proto_arc);
    std::thread::spawn(move || {
        p_clone2.connect_thread();
    });

    let p_clone3 = Arc::clone(&proto_arc);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            let server = api::ApiServer::new(Arc::clone(&p_clone3));
            server.run(8384).await
        });
    });
    let _ = PROTOCOL.set(proto_arc);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_kekulen_app_ProtocolBridge_sendMessage(
    mut env: JNIEnv,
    _class: JClass,
    pub_key: JString,
    message: JString,
) -> jstring {
    let key: String = env.get_string(&pub_key).expect("Bad key").into();
    let msg: String = env.get_string(&message).expect("Bad msg").into();

    let res_str = if let Some(proto) = PROTOCOL.get() {
        match proto.send_to(&key, &msg) {
            Ok(_) => "OK".to_string(),
            Err(e) => format!("Error: {}", e),
        }
    } else {
        "Protocol not initialized".to_string()
    };

    env.new_string(res_str).expect("String fail").into_raw()
}
