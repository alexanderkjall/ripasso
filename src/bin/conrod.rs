#[macro_use]
extern crate conrod;
mod pass;

use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use pass::Password;

extern crate clipboard;
use conrod::{widget, Positionable, Sizeable, Colorable, Widget};
use conrod::backend::glium::glium::{self, Surface};
fn main() {

    // Channel for password updates
    let (password_tx, password_rx): (Sender<Password>, Receiver<Password>) = mpsc::channel();

    // Load and watch all the passwords in the background
    pass::load_and_watch_passwords(password_tx).expect("failed to locate password directory");

    let passwords = Arc::new(Mutex::new(vec![]));
    let p1 = passwords.clone();
    thread::spawn(move || loop {
                      match password_rx.recv() {
                          Ok(p) => {
                              println!("Recieved: {:?}", p.name);
                              let mut passwords = p1.lock().unwrap();
                              passwords.push(p);
                          }
                          Err(e) => {
                              panic!("password reciever channel failed: {:?}", e);
                          }
                      }
                  });

    println!("Hello passwords");

    // UI
    const WIDTH: u32 = 400;
    const HEIGHT: u32 = 200;
    let mut events_loop = glium::glutin::EventsLoop::new();

    let window = glium::glutin::WindowBuilder::new()
        .with_title("Ripasso")
        .with_dimensions(WIDTH, HEIGHT);

    let context = glium::glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_multisampling(4);

    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let mut ui = conrod::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).build();

    widget_ids!(struct Ids { text, password_list, input });
    let mut ids = Ids::new(ui.widget_id_generator());
    //ids.passwords.resize(100, &mut ui.widget_id_generator());

    // Add a `Font` to the `Ui`'s `font::Map` from file.
    const FONT_PATH: &'static str = "/usr/share/fonts/TTF/arial.ttf";
    ui.fonts.insert_from_file(FONT_PATH).unwrap();

    let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();

    // The image map describing each of our widget->image mappings (in our case, none).
    let image_map = conrod::image::Map::<glium::texture::Texture2d>::new();


    let mut events = Vec::new();

    'render: loop {
        events.clear();

        // Get all the new events since the last frame.
        events_loop.poll_events(|event| { events.push(event); });

        // If there are no new events, wait for one.
        if events.is_empty() {
            events_loop.run_forever(|event| {
                                        events.push(event);
                                        glium::glutin::ControlFlow::Break
                                    });
        }

        fn normalized(s: &String) -> String {
            s.to_lowercase()
        };
        fn matches(s: &String, q: &String) -> bool {
            normalized(&s).as_str().contains(normalized(&q).as_str())
        };

        let all = passwords.lock().unwrap();
        let matching = all.iter()
            .filter(|p| matches(&p.name, &"wrapp".to_string())).collect();
        // Process the events.
        for event in events.drain(..) {

            // Break from the loop upon `Escape` or closed window.
            match event.clone() {
                glium::glutin::Event::WindowEvent { event, .. } => {
                    match event {
                        glium::glutin::WindowEvent::Closed |
                        glium::glutin::WindowEvent::KeyboardInput {
                            input: glium::glutin::KeyboardInput {
                                virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape), ..
                            },
                            ..
                        } => break 'render,
                        _ => (),
                    }
                }
                _ => (),
            };

            // Use the `winit` backend feature to convert the winit event to a conrod input.
            let input = match conrod::backend::winit::convert_event(event, &display) {
                None => continue,
                Some(input) => input,
            };

            // Handle the input with the `Ui`.
            ui.handle_event(input);
            // Set the widgets.
            //let ui = &mut ui.set_widgets();

        }


        set_ui(ui.set_widgets(), matching, &ids);

        // Draw the `Ui` if it has changed.
        if let Some(primitives) = ui.draw_if_changed() {
            renderer.fill(&display, primitives, &image_map);
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            renderer.draw(&display, &mut target, &image_map).unwrap();
            target.finish().unwrap();
        }
    }

    fn set_ui(ref mut ui: conrod::UiCell, passwords: Vec<&Password>, ids: &Ids) {
        widget::text_box::TextBox::new("test")
            .align_top_of(ui.window)
            .w_of(ui.window)
            .h(32.0)
            .font_size(16)
            .text_color(conrod::color::WHITE)
            .color(conrod::color::BLACK)
            .set(ids.input, ui);


        let (mut items, scrollbar ) = widget::List::flow_down(passwords.len())
            .item_size(30.0)
            .scrollbar_on_top()
            .down_from(ids.input, 10.0)
            .wh_of(ui.window)

            .set(ids.password_list, ui);

        while let Some(item) = items.next(ui){
            let i = item.i;
            let pw = passwords[i].clone();
            let label = pw.name.to_owned();
            let listItem = widget::Text::new(&label)
                .color(conrod::color::WHITE)
                .font_size(16);
            item.set(listItem, ui);
        }
        //ui.change_focus_to(ids.input);
        if let Some(s) = scrollbar { s.set(ui)}
        /*
        for (i, p) in passwords.iter().enumerate() {
            //println!("Hello: {:?} {:?}", i, p.name);
            // "Hello World!" in the middle of the screen.
            widget::Text::new(&p.name.to_owned())
                .middle_of(ui.window)
            //.align_middle_x_of(ui.window)
            //.down(10.0)
                .color(conrod::color::WHITE)
                .font_size(32)
                .set(ids.passwords[i], ui);
        }
        */
    }
}
