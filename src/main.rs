#![windows_subsystem = "windows"]

use std::fs::OpenOptions;
use std::path::PathBuf;
use app::App;
use clap::Parser;
use fd_lock::RwLock;
use gtk::{gdk, prelude::*};
use gtk::{glib, Application};
use runner::runner_config::RunnerConfig;
use shared_memory::{ShmemConf, ShmemError};

pub(crate) mod app;
mod sound_source;
mod lynx_display;
mod runner;

const APP_ID: &str = "io.github.lleny.holani-gtk";
pub(crate) const LOCK_ID: &str = "holani-gtk_lock";
pub(crate) const CART_ID: &str = "holani-gtk_cart";
pub(crate) const LOCK_SIZE: usize = 4096;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Cartridge, can be .o or a .lnx file
    #[arg(short, long)]
    cartridge: Option<PathBuf>,

    /// ROM override
    #[arg(short, long)]
    rom: Option<PathBuf>,
    
    /// Allows only one instance running
    #[arg(short, long, default_value_t = false)]
    single_instance: bool,
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

    let mut file_lock = RwLock::new(
        OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(LOCK_ID)
        .expect("Couldn't open locking file.")
    );
    let lock_guard = file_lock.try_write();
    if lock_guard.is_err() && config.single_instance() {
        post_cart_name_to_shared_mem(&config);
        mainapp.quit();
        return glib::ExitCode::SUCCESS;
    };    

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
    if let Some(rom) = args.rom {
        config.set_rom(rom);
    }
    if let Some(cart) = args.cartridge {
        config.set_cartridge(cart);
    }
    
    config.set_single_instance(args.single_instance);

    config
}

fn post_cart_name_to_shared_mem(config: &RunnerConfig) {
    match ShmemConf::new().size(LOCK_SIZE).flink(CART_ID).create() {
        Ok(_) => panic!("Shared mem doesn't exist."),
        Err(ShmemError::LinkExists) => match ShmemConf::new().flink(CART_ID).open() {
            Ok(shmem) => if let Some(cart) = config.cartridge() {
                unsafe { 
                    let raw_ptr = shmem.as_ptr();
                    let str_len = raw_ptr as *mut u32;
                    let str_data = std::slice::from_raw_parts_mut(
                        raw_ptr.add(std::mem::size_of::<u32>()),
                        shmem.len() - std::mem::size_of::<u32>()
                    );
                    let st = cart.to_str().unwrap();
                    str_data[..st.len()].copy_from_slice(st.as_bytes());
                    *str_len = st.len() as u32;
                }
            },
            Err(e) => panic!("Unable to open Shared mem flink: {}", e),
        },
        Err(e) => panic!("Unable to open Shared mem flink: {}", e)
    };
}


