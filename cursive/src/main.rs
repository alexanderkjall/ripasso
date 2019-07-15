/*  Ripasso - a simple password manager
    Copyright (C) 2018 Joakim Lundborg

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

extern crate cursive;
extern crate env_logger;
extern crate ripasso;

use self::cursive::traits::*;
use self::cursive::views::{
    Dialog, EditView, LinearLayout, OnEventView, SelectView, TextArea, TextView,
};

use cursive::Cursive;

use self::cursive::direction::Orientation;
use self::cursive::event::{Event, Key};

extern crate clipboard;
use self::clipboard::{ClipboardContext, ClipboardProvider};

use ripasso::pass;
use std::process;

fn down(ui: &mut Cursive) -> () {
    ui.call_on_id("results", |l: &mut SelectView<pass::PasswordEntry>| {
        l.select_down(1);
    });
}

fn up(ui: &mut Cursive) -> () {
    ui.call_on_id("results", |l: &mut SelectView<pass::PasswordEntry>| {
        l.select_up(1);
    });
}

fn errorbox(ui: &mut Cursive, err: &pass::Error) -> () {
    let d = Dialog::around(TextView::new(format!("{:?}", err)))
        .dismiss_button("Ok")
        .title("Error");
    ui.add_layer(d);
}

fn copy(ui: &mut Cursive) -> () {
    ui.call_on_id("results", |l: &mut SelectView<pass::PasswordEntry>| {
        let sel = l.selection();

        if sel.is_none() {
            return;
        }

        let password = sel.unwrap().password();

        if password.is_err() {
            return;
        }

        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        ctx.set_contents(password.unwrap().to_owned()).unwrap();
    });
}

fn open(ui: &mut Cursive) -> () {
    let password_entry_option: Option<Option<std::rc::Rc<ripasso::pass::PasswordEntry>>> = ui
        .call_on_id("results", |l: &mut SelectView<pass::PasswordEntry>| {
            l.selection()
        });

    let password_entry: pass::PasswordEntry = (*(match password_entry_option {
        Some(level_1) => {
            match level_1 {
                Some(level_2) => level_2,
                None => return
            }
        },
        None => return
    })).clone();

    let password = match password_entry.secret() {
        Ok(p) => p,
        Err(_e) => return
    };
    let d =
        Dialog::around(TextArea::new().content(password).with_id("editbox"))
            .button("Edit", move |s| {
                let new_password = s
                    .call_on_id("editbox", |e: &mut TextArea| {
                        e.get_content().to_string()
                    }).unwrap();
                let r = password_entry.update(new_password);
                if let Err(e) = r {
                    errorbox(s, &e)
                }
            }).dismiss_button("Ok");

    ui.add_layer(d);
}

fn view_persons(ui: &mut Cursive) -> () {
    let signers : Vec<ripasso::pass::Signer> = ripasso::pass::Signer::all_signers();

    let mut persons = SelectView::<pass::Signer>::new().h_align(cursive::align::HAlign::Left);

    for signer in signers {
        persons.add_item(format!("{} {}",signer.key_id.clone(), signer.name.clone()), signer);
    }

    let d = Dialog::around(persons).dismiss_button("Ok");

    ui.add_layer(d);
}

fn search(passwords: &pass::PasswordList, ui: &mut Cursive, query: &str) -> () {
    let col = ui.screen_size().x;
    ui.call_on_id("results", |l: &mut SelectView<pass::PasswordEntry>| {
        let r = pass::search(&passwords, &String::from(query));
        l.clear();
        for p in &r {
            let label = format!(
                            "{:2$}  {}",
                            p.name,
                            match p.updated {
                                Some(d) => format!("{}", d.format("%Y-%m-%d")),
                                None => "n/a".to_string(),
                            },
                            _ = col - 10 - 8, // Optimized for 80 cols
                        );
            l.add_item(label, p.clone());
        }
    });
}

fn main() {
    env_logger::init();

    // Load and watch all the passwords in the background
    let (password_rx, passwords) = match pass::watch() {
        Ok(t) => t,
        Err(e) => {
            println!("Error {:?}", e);
            process::exit(1);
        }
    };

    let mut ui = Cursive::default();

    // Update UI on password change event
    let e = ui.cb_sink().send(Box::new(move |s: &mut Cursive| {
        let event = password_rx.try_recv();
        if let Ok(e) = event {
            if let pass::PasswordEvent::Error(ref err) = e {
                errorbox(s, err)
            }
        }
    }));

    if e.is_err() {
        eprintln!("Application error: {}", e.err().unwrap());
        return;
    }

    ui.add_global_callback(Event::CtrlChar('y'), copy);
    ui.add_global_callback(Key::Enter, copy);

    // Movement
    ui.add_global_callback(Event::CtrlChar('n'), down);
    ui.add_global_callback(Event::CtrlChar('p'), up);

    // View list of persons that have access
    ui.add_global_callback(Event::CtrlChar('v'), view_persons);

    // Query editing
    ui.add_global_callback(Event::CtrlChar('w'), |ui| {
        ui.call_on_id("searchbox", |e: &mut EditView| {
            e.set_content("");
        });
    });

    // Editing
    ui.add_global_callback(Event::CtrlChar('o'), open);

    ui.add_global_callback(Event::Key(cursive::event::Key::Esc), |s| s.quit());

    ui.load_toml(include_str!("../res/style.toml")).unwrap();
    let searchbox = EditView::new()
        .on_edit(move |ui: &mut cursive::Cursive, query, _| {
            search(&passwords, ui, query)
        }).with_id("searchbox")
        .fixed_width(72);

    // Override shortcuts on search box
    let searchbox = OnEventView::new(searchbox)
        .on_event(Key::Up, up)
        .on_event(Key::Down, down);

    let results = SelectView::<pass::PasswordEntry>::new()
        .with_id("results")
        .full_height();

    ui.add_layer(
        LinearLayout::new(Orientation::Vertical)
            .child(
                Dialog::around(
                    LinearLayout::new(Orientation::Vertical)
                        .child(searchbox)
                        .child(results)
                        .fixed_width(72),
                ).title("Ripasso"),
            ).child(
                LinearLayout::new(Orientation::Horizontal)
                    .child(TextView::new("CTRL-N: Next "))
                    .child(TextView::new("CTRL-P: Previous "))
                    .child(TextView::new("CTRL-Y: Copy "))
                    .child(TextView::new("CTRL-W: Clear "))
                    .child(TextView::new("CTRL-O: Open "))
                    .child(TextView::new("CTRL-V: View Signers "))
                    .child(TextView::new("esc: Quit"))
                    .full_width(),
            ),
    );
    ui.run();
}
