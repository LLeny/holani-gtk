#![windows_subsystem = "windows"]

use std::path::PathBuf;
use app::App;
use clap::Parser;
use gtk::{gdk, prelude::*};
use gtk::{glib, Application};
use runner::runner_config::RunnerConfig;

pub(crate) mod app;
mod lynx_display;
mod runner;

const APP_ID: &str = "io.github.lleny.holani-gtk";

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Cartridge, can be .o or a .lnx file
    #[arg(short, long)]
    cartridge: Option<PathBuf>,
}

pub(crate) enum Event {
    UpdateDisplay(Vec<u8>),
    UpdateConfig(RunnerConfig),
    LoadCart(PathBuf),
    LoadROM(PathBuf),
    ReloadCart,
    LoadState(PathBuf),
    SaveState(PathBuf),
    Pause(bool),
    Reset,
    Mute(bool),
    KeyPressed(gdk::Key),
    KeyReleased(gdk::Key),
    About,
    Quit,
}

fn main() -> glib::ExitCode {  
    env_logger::init(); 
    let mainapp = Application::builder().application_id(APP_ID).build();
    let config = process_args();
    let qapp = mainapp.clone();

    mainapp.connect_activate(move |app| {
        let lapp = qapp.clone();
        let (event_tx, event_rx) = kanal::unbounded::<Event>();
        let mut app = App::new(app, event_tx, config.clone());

        let event_handler = async move {
            while let Ok(event) = event_rx.as_async().recv().await {
                match event {
                    Event::UpdateDisplay(buffer) => app.setup_next_frame(&buffer),
                    Event::UpdateConfig(config) => app.set_new_config(config),
                    Event::LoadCart(file) => app.load_cart(file),
                    Event::LoadROM(file) => app.load_rom(file),
                    Event::ReloadCart => app.reload_cart(),
                    Event::LoadState(file) => app.load_state(file),
                    Event::SaveState(file) => app.save_state(file),
                    Event::Pause(p) => app.pause(p),
                    Event::Reset => app.reset(),
                    Event::Mute(m) => app.mute(m),
                    Event::About => app.show_about(),
                    Event::Quit => lapp.quit(),
                    Event::KeyPressed(key) => app.key_pressed(key),
                    Event::KeyReleased(key) => app.key_released(key),
                }
            }
        };

        glib::MainContext::default().spawn_local(event_handler);
    });
    
    mainapp.run_with_args(&[""])
}

fn process_args() -> RunnerConfig {
    let args = Args::parse();

    let mut config = RunnerConfig::default();
    if let Some(cart) = args.cartridge {
        config.set_cartridge(cart);
    }

    config
}



