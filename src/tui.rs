#[allow(unused_imports)]
use crate::sam::sam::*;
//use sam::sam::{SAM,KeyPair};
use cursive::views::{Dialog, TextView, EditView, SelectView, LinearLayout};
use cursive::Cursive;
use cursive::view::Nameable;
use cursive::view::Resizable;
use std::fs;


// by gemini modified
fn create_account_dialog(s: &mut Cursive) {
    s.add_layer(
        Dialog::new()
        .title("New Account Name")
        .content(
            EditView::new()
            .with_name("new_acc_name")
            .fixed_width(20)
        )
        .button("Create", |s| {
            let name = s.call_on_name("new_acc_name", |v: &mut EditView| v.get_content()).unwrap();
            if !name.is_empty() {

                let name_str = name.to_string();
                let key_pair = SAM::new("127.0.0.1", 7656).generate_dest(7) ;
                key_pair.save_to_file(&name);

                s.call_on_name("select", |view: &mut SelectView<String>| {
                    view.add_item_str(name_str);
                });
                s.pop_layer();
            }
        })
        .button("Cancel", |s| { s.pop_layer(); })
    );
}
fn add_friend_dialog(s: &mut Cursive) {
    s.add_layer(
        Dialog::new()
        .title("Add friend")
        .content(
            LinearLayout::vertical()
            .child(TextView::new("Name (show name):"))
            .child(EditView::new().with_name("new_friend_name").fixed_width(30))
            .child(TextView::new("I2P B64 address:"))
            .child(EditView::new().with_name("new_friend_key").fixed_width(30))
        )
        .button("Add", |s| {
            let name = s.call_on_name("new_friend_name", |v: &mut EditView| {
                v.get_content()
            }).map(|rc| rc.to_string()).unwrap_or_default();

            let key = s.call_on_name("new_friend_key", |v: &mut EditView| {
                v.get_content()
                //todo!();
            }).map(|rc| rc.to_string()).unwrap_or_default();

            if !name.is_empty() && key.len() > 50 {
                s.call_on_name("friends_list", |v: &mut SelectView<String>| {
                    v.add_item(name, key);
                });
                s.pop_layer();
            } else {
                s.add_layer(Dialog::info("Wrong data."));
            }
        })
        .button("Abort", |s| { s.pop_layer(); })
    );
}

fn InitMessenger( s: &mut Cursive, keyPair: KeyPair ) {
    s.pop_layer();
    s.add_layer(
        Dialog::around(
            LinearLayout::vertical()
            .child(TextView::new("Select your friend:"))
            //   .child(select.with_name("select"))
        )
        .title("Kekulen I2P Chat")
        .button("Add new", |s| add_friend_dialog(s))
        .button("Quit", |s| s.quit())
    );
}

pub fn run() {
    let mut siv = cursive::default();
    let dir = KeyPair::get_app_dir();

    // by gemini modified
    let mut select = SelectView::<String>::new().on_submit(move |s, name: &str| {

        let n = KeyPair::load_from_file(name).expect("Cant init keypair");

        InitMessenger(s,n);
    });

    // by gemini
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "dat" {
                    if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                        select.add_item_str(name);
                    }
                }
            }
        }
    }

    // by gemini
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
            .child(TextView::new("Select your account:"))
            .child(select.with_name("select"))
        )
        .title("Kekulen I2P Chat")
        .button("Create new", |s| create_account_dialog(s))
        .button("Quit", |s| s.quit())
    );

    siv.run();
}
