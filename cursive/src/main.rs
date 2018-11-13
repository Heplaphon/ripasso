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

use std::sync::Mutex;
fn main() {
    env_logger::init();

    // Load and watch all the passwords in the background
    let (_password_rx, passwords) = match pass::watch() {
        Ok(t) => t,
        Err(e) => {
            println!("Error {:?}", e);
            process::exit(1);
        }
    };

    let mut ui = Cursive::default();
    let rrx = Mutex::new(_password_rx);

    fn errorbox(ui: &mut Cursive, err: &pass::Error) -> () {
        let d = Dialog::around(TextView::new(format!("{:?}", err)))
            .dismiss_button("Ok")
            .title("Error");
        ui.add_layer(d);
    }

    ui.cb_sink().send(Box::new(move |s: &mut Cursive| {
        let event = rrx.lock().unwrap().try_recv();
        match event {
            Ok(e) => match e {
                pass::PasswordEvent::Error(ref err) => errorbox(s, err),
                _ => (),
            },
            _ => (),
        }
    }));

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

    // Copy
    fn copy(ui: &mut Cursive) -> () {
        ui.call_on_id("results", |l: &mut SelectView<pass::PasswordEntry>| {
            let password = l.selection().unwrap().password().unwrap();
            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
            ctx.set_contents(password.to_owned()).unwrap();
        });
    };
    ui.add_global_callback(Event::CtrlChar('y'), copy);
    ui.add_global_callback(Key::Enter, copy);

    // Movement
    ui.add_global_callback(Event::CtrlChar('n'), down);
    ui.add_global_callback(Event::CtrlChar('p'), up);

    // Query editing
    ui.add_global_callback(Event::CtrlChar('w'), |ui| {
        ui.call_on_id("searchbox", |e: &mut EditView| {
            e.set_content("");
        });
    });

    // Editing
    ui.add_global_callback(Event::CtrlChar('o'), |ui| {
        let password_entry: pass::PasswordEntry = (*ui
            .call_on_id("results", |l: &mut SelectView<pass::PasswordEntry>| {
                l.selection().unwrap()
            })
            .unwrap())
        .clone();

        let password = password_entry.secret().unwrap();
        let d = Dialog::around(
            TextArea::new().content(password).with_id("editbox"),
        )
        .button("Edit", move |s| {
            let new_password = s
                .call_on_id("editbox", |e: &mut TextArea| {
                    e.get_content().to_string()
                })
                .unwrap();
            let r = password_entry.update(new_password);
            match r {
                Err(e) => errorbox(s, &e),
                Ok(_) => (),
            }
        })
        .dismiss_button("Ok");

        ui.add_layer(d);
    });

    ui.load_toml(include_str!("../res/style.toml")).unwrap();
    let searchbox = EditView::new()
        .on_edit(move |ui, query, _| {
            let col = ui.screen_size().x;
            ui.call_on_id(
                "results",
                |l: &mut SelectView<pass::PasswordEntry>| {
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
                },
            );
        })
        .with_id("searchbox")
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
                )
                .title("Ripasso"),
            )
            .child(
                LinearLayout::new(Orientation::Horizontal)
                    .child(TextView::new("CTRL-N: Next "))
                    .child(TextView::new("CTRL-P: Previous "))
                    .child(TextView::new("CTRL-Y: Copy "))
                    .child(TextView::new("CTRL-W: Clear "))
                    .child(TextView::new("CTRL-O: Open"))
                    .full_width(),
            ),
    );
    ui.run();
}
