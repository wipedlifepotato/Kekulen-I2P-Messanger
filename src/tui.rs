/*
 *  by gemini
 * */
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cursive::Cursive;
use cursive::theme::{BaseColor, Color, ColorStyle, Effect};
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, DummyView, EditView, LinearLayout, ScrollView, SelectView, TextView};

use crate::kekulenprot::Protocol;

// Храним состояние нашего интерфейса
#[derive(Clone)]
struct AppState {
    protocol: Arc<Protocol>,
    selected_friend_pub: Arc<Mutex<Option<String>>>,
}

pub fn run(protocol: Arc<Protocol>) {
    let mut siv = cursive::default();

    // Инициализируем стейт
    let state = AppState {
        protocol: Arc::clone(&protocol),
        selected_friend_pub: Arc::new(Mutex::new(None)),
    };
    siv.set_user_data(state.clone());

    // --- ЛЕВАЯ ПАНЕЛЬ: Список друзей ---
    let friends_list = SelectView::<String>::new()
        .on_select(|s: &mut Cursive, pub_key: &String| {
            let state = s.user_data::<AppState>().expect("AppState missing").clone();
            *state.selected_friend_pub.lock().unwrap() = Some(pub_key.clone());
            update_messages_view(s);
        })
        .on_submit(|s: &mut Cursive, pub_key: &String| {
            let state = s.user_data::<AppState>().expect("AppState missing").clone();
            *state.selected_friend_pub.lock().unwrap() = Some(pub_key.clone());
            update_messages_view(s);
            // Исправленный фокус: просто игнорируем результат через let _
            let _ = s.focus_name("msg_input");
        })
        .with_name("friends_list")
        .fixed_width(35);

    let sidebar = LinearLayout::vertical()
        .child(TextView::new("Kekulen TUI").effect(Effect::Bold))
        .child(
            TextView::new(format!("My ID: {}...", &protocol.get_my_public_key()[..16]))
                .style(Color::Dark(BaseColor::Cyan)),
        )
        .child(DummyView)
        .child(Dialog::around(ScrollView::new(friends_list)).title("Friends"));

    // --- ПРАВАЯ ПАНЕЛЬ: Окно чата ---
    let messages_view = ScrollView::new(
        TextView::new("Select a friend to start chatting\n").with_name("messages_text"),
    )
    .scroll_strategy(cursive::view::ScrollStrategy::StickToBottom)
    .with_name("messages_scroll")
    .full_width()
    .full_height();

    let input_view = EditView::new()
        .on_submit(|s, text| {
            send_message(s, text);
        })
        .with_name("msg_input")
        .full_width();

    let chat_area = LinearLayout::vertical()
        .child(messages_view)
        .child(DummyView)
        .child(
            LinearLayout::horizontal()
                .child(TextView::new("Msg: "))
                .child(input_view),
        );

    let chat_dialog = Dialog::around(chat_area).title("Chat").full_screen();

    // --- ГЛАВНЫЙ СЛОЙ ---
    siv.add_layer(
        LinearLayout::horizontal()
            .child(
                LinearLayout::vertical()
                    .child(sidebar)
                    .child(cursive::views::Button::new("Add Friend", add_friend_dialog))
                    .child(cursive::views::Button::new("Quit", |s| s.quit())),
            )
            .child(DummyView)
            .child(chat_dialog),
    );

    // --- ФОНОВЫЙ ПОТОК ОБНОВЛЕНИЯ ---
    let cb_sink = siv.cb_sink().clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(500));
            let _ = cb_sink.send(Box::new(|s: &mut Cursive| {
                update_friends_list(s);
                update_messages_view(s);
            }));
        }
    });

    siv.run();
}

// --- ФУНКЦИИ ЛОГИКИ ИНТЕРФЕЙСА ---

fn send_message(s: &mut Cursive, text: &str) {
    if text.trim().is_empty() {
        return;
    }

    let state = s.user_data::<AppState>().unwrap().clone();
    let current_friend = state.selected_friend_pub.lock().unwrap().clone();

    if let Some(pub_key) = current_friend {
        // Пытаемся отправить сообщение через протокол
        match state.protocol.send_to(&pub_key, text) {
            Ok(_) => {
                s.call_on_name("msg_input", |v: &mut EditView| {
                    v.set_content("");
                });
                update_messages_view(s);
            }
            Err(e) => {
                // Если ключи еще не обменяны или ошибка I2P
                s.add_layer(Dialog::info(format!("Cannot send: {}", e)));
            }
        }
    }
}

fn update_friends_list(s: &mut Cursive) {
    let state = s.user_data::<AppState>().unwrap().clone();
    let friends_mtx = state.protocol.get_friends_list();
    let friends = friends_mtx.lock().unwrap();

    let current_selection = state.selected_friend_pub.lock().unwrap().clone();

    s.call_on_name("friends_list", |list: &mut SelectView<String>| {
        // Если список не изменился по размеру и ключам, можно не перерисовывать так часто,
        // но для простоты просто сохраняем позицию.
        let old_selection = list.selected_id();

        list.clear();
        for f in friends.iter() {
            let status = if f.is_active { "[Online]" } else { "[Offline]" };
            // Проверяем наличие ключей через Option::is_some()
            let pqc = if f.key_exchanged { "🔐" } else { "⏳" };

            let display_text = format!("{} {} {}", pqc, status, &f.name);
            list.add_item(display_text, f.pub_key.clone());
        }

        // Восстанавливаем выделение по публичному ключу
        if let Some(ref sel_pub) = current_selection {
            if let Some(idx) = friends.iter().position(|f| &f.pub_key == sel_pub) {
                list.set_selection(idx);
            }
        } else if let Some(id) = old_selection {
            list.set_selection(id);
        }
    });
}

fn update_messages_view(s: &mut Cursive) {
    let state = s.user_data::<AppState>().unwrap().clone();
    let current_friend_pub = state.selected_friend_pub.lock().unwrap().clone();

    if let Some(pub_key) = current_friend_pub {
        let friends_mtx = state.protocol.get_friends_list();
        let friends = friends_mtx.lock().unwrap();

        if let Some(f) = friends.iter().find(|f| f.pub_key == pub_key) {
            let mut chat_content = StyledString::new();
            for msg in &f.messages {
                if msg.starts_with("Me: ") {
                    chat_content.append(StyledString::styled(
                        format!("{}\n", msg),
                        ColorStyle::new(Color::Dark(BaseColor::Green), Color::TerminalDefault),
                    ));
                } else {
                    chat_content.append(StyledString::styled(
                        format!("{}\n", msg),
                        ColorStyle::new(Color::Light(BaseColor::White), Color::TerminalDefault),
                    ));
                }
            }

            s.call_on_name("messages_text", |v: &mut TextView| {
                v.set_content(chat_content);
            });

            // Принудительная прокрутка вниз при новых сообщениях
            s.call_on_name("messages_scroll", |v: &mut ScrollView<TextView>| {
                v.scroll_to_bottom();
            });
        }
    }
}

fn add_friend_dialog(s: &mut Cursive) {
    s.add_layer(
        Dialog::new()
            .title("Add Friend")
            .content(
                LinearLayout::vertical()
                    .child(TextView::new("Name:"))
                    .child(EditView::new().with_name("new_friend_name").fixed_width(40))
                    .child(TextView::new("I2P B64 Destination:"))
                    .child(EditView::new().with_name("new_friend_key").fixed_width(40)),
            )
            .button("Add", |s| {
                let name = s
                    .call_on_name("new_friend_name", |v: &mut EditView| v.get_content())
                    .unwrap()
                    .to_string();
                let pub_key = s
                    .call_on_name("new_friend_key", |v: &mut EditView| v.get_content())
                    .unwrap()
                    .to_string();

                if !name.is_empty() && pub_key.len() > 50 {
                    let state = s.user_data::<AppState>().unwrap().clone();
                    state.protocol.add_pending_friend(&name, &pub_key);

                    update_friends_list(s);
                    s.pop_layer();
                } else {
                    s.add_layer(Dialog::info("Invalid Name or B64 Key (too short)"));
                }
            })
            .button("Cancel", |s| {
                s.pop_layer();
            }),
    );
}
