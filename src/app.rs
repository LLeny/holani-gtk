use std::path::PathBuf;

use gtk::{gdk, gio::{self, Menu, MenuItem, MenuModel, SimpleAction}, glib::{self, clone}, prelude::{ActionExt, ActionMapExtManual, ApplicationExt, BoxExt, ButtonExt, FileExt, GridExt, GtkApplicationExt, GtkWindowExt, ObjectExt, ToValue, WidgetExt}, AlertDialog, Application, ApplicationWindow, FileFilter, LayoutManager, PopoverMenu, PopoverMenuBar};
use holani::suzy::registers::{Joystick, Switches};
use log::error;
use strum::IntoEnumIterator;
use crate::{lynx_display::LynxDisplay, runner::{runner_config::{Input, RunnerAction, RunnerConfig, RunnerStatus}, Runner}, Event};

macro_rules! btn_event {
    ($win: ident, $event_tx: expr, $cmd: expr, $mne: expr, $evt: expr) => {
        let tx = $event_tx.clone();
        let a = gio::ActionEntry::builder($cmd)
            .activate(clone!(
                #[strong] tx,
                move |_: &gtk::Application, _, _| {
                    tx.send($evt).unwrap();
                })
            )
            .build();
        $win.application().unwrap().set_accels_for_action(format!("app.{}", $cmd).as_str(), &[$mne]);    
        $win.application().unwrap().add_action_entries([a]);
    };
}


pub struct App {
    display: LynxDisplay,
    runner: Runner,
    config: RunnerConfig,
    input_tx: kanal::Sender<(u8, u8)>,
    config_tx: kanal::Sender<RunnerConfig>,
    event_tx: kanal::Sender<Event>,
    joy: Joystick,
    switches: Switches,
}

impl App {
    pub fn new(app: &gtk::Application, event_tx: kanal::Sender<Event>) -> Self {
        
        let mut config = match confy::load::<RunnerConfig>("holani-gtk", None) {
            Err(e) => {
                error!("Couldn't load settings. Using defaults. '{}'", e);
                RunnerConfig::default()
            }
            Ok(s) => s,
        };

        let mut runner = Runner::new();

        let (input_tx, config_tx, rotation) = runner.initialize_thread(event_tx.clone(), config.clone());

        config.set_rotation(rotation);

        let mut slf = Self {
            display: LynxDisplay::default(),
            runner,
            config,          
            input_tx,
            config_tx,
            event_tx,
            joy: Joystick::empty(),
            switches: Switches::empty(),
        };

        slf.build_ui(app);

        slf
    }

    pub fn setup_next_frame(&mut self, data: &Vec<u8>) {
        self.display.setup_next_frame(data);
    }

    pub fn set_new_config(&mut self, config: RunnerConfig) {
        self.config = config;
        self.update_config();
    }

    fn key_changed(&mut self, key: gdk::Key, value: bool) {
        let kstr = key.name().unwrap().to_lowercase();
        if !self.config.button_mapping().contains_key(&kstr) {
            return;
        }
        
        match self.config.button_mapping()[&kstr] {
            Input::Up => self.joy.set(Joystick::up, value),
            Input::Down => self.joy.set(Joystick::down, value),
            Input::Left => self.joy.set(Joystick::left, value),
            Input::Right => self.joy.set(Joystick::right, value),
            Input::Outside => self.joy.set(Joystick::outside, value),
            Input::Inside => self.joy.set(Joystick::inside, value),
            Input::Option1 => self.joy.set(Joystick::option_1, value),
            Input::Option2 => self.joy.set(Joystick::option_2, value),
            Input::Pause => self.switches.set(Switches::pause, value),
        };
        self.input_tx.send((self.joy.bits(), self.switches.bits())).unwrap();
    }

    pub fn key_pressed(&mut self, key: gdk::Key) {
        self.key_changed(key, true);
    }

    pub fn key_released(&mut self, key: gdk::Key) {
        self.key_changed(key, false);
    }

    fn build_ui(&mut self, app: &Application) {

        let picture = gtk::Picture::builder()
            .paintable(&self.display)
            .hexpand(true)
            .vexpand(true)
            .halign(gtk::Align::Fill)
            .valign(gtk::Align::Fill)
            .content_fit(gtk::ContentFit::Contain)
            .can_shrink(true)
            .build();

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Holani")
            .child(&picture)
            .show_menubar(true)
            .build();    

        let event_controller = gtk::EventControllerKey::new();

        let tx = self.event_tx.clone();
        event_controller.connect_key_pressed(clone!(
            #[strong] tx,
            move |_, key, _, _| {
                tx.send(Event::KeyPressed(key)).unwrap();
                glib::Propagation::Stop
            }));
        
        event_controller.connect_key_released(clone!(
            #[strong] tx,
            move |_, key, _, _| {
                tx.send(Event::KeyReleased(key)).unwrap();
            }));

        window.add_controller(event_controller); 

        self.build_menu(&window);
        
        window.present();        
    }

    fn build_menu(&self,  window: &gtk::ApplicationWindow) {
    
        btn_event!(window, self.event_tx, "about", "<Alt>a", Event::About);
        btn_event!(window, self.event_tx, "exit", "<Alt>x", Event::Quit);
        btn_event!(window, self.event_tx, "reload_cart", "<Alt>r", Event::ReloadCart);
        btn_event!(window, self.event_tx, "reset", "<Alt>t", Event::Reset);
        
        let tx = self.event_tx.clone();
        let app = window.application().unwrap();

        let menubar = {
            let file_menu = {            
                let load_cart_menu_item = gio::MenuItem::new(Some("Load _cart"), Some("app.load_cart"));
                let reload_cart_menu_item = gio::MenuItem::new(Some("_Reload cart"), Some("app.reload_cart"));
                let load_state_menu_item = gio::MenuItem::new(Some("_Load state"), Some("app.load_state"));
                let save_state_item = gio::MenuItem::new(Some("_Save state"), Some("app.save_state"));
                let quit_menu_item = gio::MenuItem::new(Some("E_xit"), Some("app.exit"));
   
                let load_cart_action = gio::ActionEntry::builder("load_cart")
                    .activate(clone!(
                        #[strong] tx,
                        #[weak] window,
                        move |_, _, _| show_cart_picker(tx.clone(), &window)
                    ))
                    .build();

                let load_state_action = gio::ActionEntry::builder("load_state")
                    .activate(clone!(
                        #[strong] tx,
                        #[weak] window,
                        move |_, _, _| show_state_picker(tx.clone(), &window)
                    ))
                    .build();

                let save_state_action = gio::ActionEntry::builder("save_state")
                    .activate(clone!(
                        #[strong] tx,
                        #[weak] window,
                        move |_, _, _| show_state_writer(tx.clone(), &window)
                    ))
                    .build();

                app.add_action_entries([load_cart_action, load_state_action, save_state_action]);
                app.set_accels_for_action("app.load_cart", &["<Alt>c"]); 
                app.set_accels_for_action("app.load_state", &["<Alt>l"]); 
                app.set_accels_for_action("app.save_state", &["<Alt>s"]); 

                let file_menu = gio::Menu::new();
                file_menu.append_item(&load_cart_menu_item);
                file_menu.append_item(&reload_cart_menu_item);
                let state_menu = gio::Menu::new();
                state_menu.append_item(&load_state_menu_item);
                state_menu.append_item(&save_state_item);
                file_menu.append_section(None, &state_menu);
                let exit_menu = gio::Menu::new();
                exit_menu.append_item(&quit_menu_item);
                file_menu.append_section(None, &exit_menu);
                file_menu
            };
    
            let settings_menu = {
                let pause_menu_item = gio::MenuItem::new(Some("_Pause"), Some("app.pause"));
                let mute_state_item = gio::MenuItem::new(Some("_Mute"), Some("app.mute"));
                let keys_menu_item = gio::MenuItem::new(Some("_Buttons mapping"), Some("app.buttons"));
                let rom_header = match self.config.rom() {
                    None => "R_OM (Free Boot)".to_string(),
                    Some(path) => format!("R_OM ({:?})", path.file_name().unwrap()),
                };
                let rom_menu_item = gio::MenuItem::new(Some(&rom_header), Some("app.load_rom"));                
                let reset_menu_item = gio::MenuItem::new(Some("Rese_t"), Some("app.reset"));

                let load_rom_action = gio::ActionEntry::builder("load_rom")
                    .activate(clone!(
                        #[strong] tx,
                        #[weak] window,
                        move |_, _, _| show_rom_picker(tx.clone(), &window)
                    ))
                    .build();

                let mute_action = gio::ActionEntry::builder("mute")
                    .state(self.config.mute().into())
                    .activate(clone!(
                        #[strong] tx,
                        move |_, action, _| {
                            let checked = !action.state().unwrap().get::<bool>().unwrap();
                            action.set_state(&checked.into());
                            tx.send(Event::Mute(checked)).unwrap();
                        })
                    )
                    .build();

                let pause_action = gio::ActionEntry::builder("pause")
                    .state((self.config.status() == RunnerStatus::Paused).into())
                    .activate(clone!(
                        #[strong] tx,
                        move |_, action, _| {
                            let checked = !action.state().unwrap().get::<bool>().unwrap();
                            action.set_state(&checked.into());
                            tx.send(Event::Pause(checked)).unwrap();
                        })
                    )
                    .build();
                let cfg = self.config.clone();
                let keys_action = gio::ActionEntry::builder("buttons")
                    .activate(clone!(
                        #[strong] tx,
                        move |_, _, _| show_key_mapping_setter(tx.clone(), cfg.clone())
                    ))
                    .build();

                app.add_action_entries([pause_action, mute_action, keys_action, load_rom_action]);
                app.set_accels_for_action("app.mute", &["<Alt>m"]);
                app.set_accels_for_action("app.pause", &["<Alt>p"]);  
                app.set_accels_for_action("app.buttons", &["<Alt>b"]);
                app.set_accels_for_action("app.load_rom", &["<Alt>o"]);

                let settings_menu = gio::Menu::new();
                settings_menu.append_item(&pause_menu_item);
                settings_menu.append_item(&mute_state_item);
                let keys_menu = gio::Menu::new();
                keys_menu.append_item(&keys_menu_item);
                settings_menu.append_section(None, &keys_menu);
                let reset_menu = gio::Menu::new();
                reset_menu.append_item(&rom_menu_item);
                reset_menu.append_item(&reset_menu_item);
                settings_menu.append_section(None, &reset_menu);
                settings_menu
            };
    
            let help_menu = {
                let about_menu_item = gio::MenuItem::new(Some("_About"), Some("app.about"));
    
                let help_menu = gio::Menu::new();
                help_menu.append_item(&about_menu_item);
                help_menu
            };
    
            let menubar = gio::Menu::new();
            menubar.append_submenu(Some("_File"), &file_menu);
            menubar.append_submenu(Some("_Settings"), &settings_menu);
            menubar.append_submenu(Some("_Help"), &help_menu);
    
            menubar
        };
    
        app.set_menubar(Some(&menubar));
    }
    
    pub fn show_about(&self) {
        let dialog = gtk::AboutDialog::builder()
            .modal(true)
            .program_name("Holani")
            .version(env!("CARGO_PKG_VERSION"))
            .website("https://github.com/LLeny/holani-gtk")
            .license_type(gtk::License::Gpl30)
            .authors(["https://github.com/LLeny"])
            .build();
    
        dialog.present();
    }

    fn update_config(&mut self) {
        self.config_tx.send(self.config.clone()).unwrap();
        self.config.take_action();
        match confy::store("holani-gtk", None, &self.config) {
            Ok(_) => (),
            Err(e) => error!("Couldn't save setings. '{}'", e),
        };
    }

    pub fn mute(&mut self, mute: bool) {
        self.config.set_mute(mute);
        self.update_config();
    }

    pub fn pause(&mut self, pause: bool) {
        self.config.set_status(match pause {
            true => RunnerStatus::Paused,
            false => RunnerStatus::Running
        });
        self.update_config();
    }

    pub fn reload_cart(&mut self) {
        self.config.set_action(RunnerAction::LoadCart);
        self.update_config();
    }

    pub fn load_cart(&mut self, file: PathBuf) {
        self.config.set_cartridge(file);
        self.config.set_action(RunnerAction::LoadCart);
        self.update_config();
    }

    pub fn load_rom(&mut self, file: PathBuf) {
        self.config.set_rom(file);
        self.config.set_action(RunnerAction::LoadROM);
        self.update_config();
    }

    pub fn reset(&mut self) {
        self.config.set_action(RunnerAction::Reset);
        self.update_config();
    }

    pub fn load_state(&mut self, file: PathBuf) {
        self.config.set_action(RunnerAction::LoadState(file));
        self.update_config();
    }

    pub fn save_state(&mut self, file: PathBuf) {
        self.config.set_action(RunnerAction::SaveState(file));
        self.update_config();
    }
}

fn show_key_mapping_setter(event_tx: kanal::Sender<Event>, config: RunnerConfig) {
    let grid = gtk::Grid::builder()
        .margin_start(6).margin_end(6).margin_top(6).margin_bottom(6)
        .halign(gtk::Align::Start).valign(gtk::Align::Center)
        .row_spacing(6).column_spacing(6)
        .column_homogeneous(true)
        .build();

    for (i, input) in Input::iter().enumerate() {
        let label = gtk::Label::new(Some(input.to_string().as_str()));
        grid.attach(&label, 0, i as i32, 1, 1);

        let prev_key = config.button_mapping().iter().find(|(_, v)| **v == input).unwrap().0;
        let btn = gtk::Button::with_label(prev_key);
        unsafe { 
            btn.set_data("input", input.to_string());
            btn.set_data("key",prev_key.to_string());
        };

        btn.connect_clicked(clone!(
            #[strong] prev_key,
            #[weak] btn,
            move |_| {
            let grid = gtk::Grid::builder()
                .margin_start(6).margin_end(6).margin_top(6).margin_bottom(6)
                .halign(gtk::Align::Start).valign(gtk::Align::Center)
                .row_spacing(6).column_spacing(6)
                .column_homogeneous(true)
                .build();

            let label = gtk::Label::new(Some(prev_key.as_str()));
            grid.attach(&label, 0, 0, 2, 1);

            let btn_ok = gtk::Button::with_label("OK");
            grid.attach(&btn_ok, 0, 1, 1, 1);
        
            let btn_cancel = gtk::Button::with_label("Cancel");
            grid.attach(&btn_cancel, 1, 1, 1, 1);
        
            let keygrab = gtk::Window::builder()
                .title("Press a key")
                .child(&grid)
                .modal(true)
                .build();

            btn_cancel.connect_clicked(clone!(
                #[weak] keygrab,
                move |_| keygrab.close()
            ));

            btn_ok.connect_clicked(clone!(
                #[weak] keygrab,
                #[weak] label,
                #[weak] btn,
                move |_| {
                    unsafe { btn.set_data("key",label.text().to_string()) };
                    btn.set_label(&label.text());
                    keygrab.close()
                }
            ));

            let event_controller = gtk::EventControllerKey::new();

            event_controller.connect_key_pressed(clone!(
                move |_, key, _, _| {          
                    label.set_text(key.name().unwrap().to_lowercase().as_str());
                    glib::Propagation::Stop
                }));
            
            keygrab.add_controller(event_controller); 

            keygrab.present();
        }));
        grid.attach(&btn, 1, i as i32, 1, 1);
    }

    let max = Input::iter().count() as i32 + 1;

    let btn_ok = gtk::Button::with_label("OK");
    grid.attach(&btn_ok, 0, max, 1, 1);

    let btn_cancel = gtk::Button::with_label("Cancel");
    grid.attach(&btn_cancel, 1, max, 1, 1);
    
    let win = ApplicationWindow::builder()
        .modal(true)
        .title("Buttons")
        .child(&grid)
        .build(); 

    btn_cancel.connect_clicked(clone!(
        #[weak] win,
        move |_| win.close()
    ));   

    btn_ok.connect_clicked(clone!(
        #[weak] win,
        #[strong] config,
        move |_| {
            let mut mut_conf = config.clone();
            for child_row in 0..max-1 {
                let child = grid.child_at(1, child_row).unwrap();
                unsafe { 
                    let skey = child.data::<String>("key").unwrap().read(); 
                    let sinput = child.data::<String>("input").unwrap().read(); 
                    let input = Input::iter().find(|i| i.to_string() == sinput).unwrap();
                    mut_conf.set_button_mapping_as_str(skey, input);
                };
            }
            event_tx.send(Event::UpdateConfig(mut_conf)).unwrap();
            win.close();
        }
    ));

    win.present();
}

fn show_rom_picker(event_tx: kanal::Sender<Event>, window: &ApplicationWindow) {

    let filedialog = gtk::FileDialog::builder()
        .title("Load ROM")
        .modal(true)
        .build();

    let txc = event_tx.clone();
    filedialog.open(Some(window), gio::Cancellable::NONE, move |file| {
        if let Ok(file) = file {
            let filename = file.path().expect("Couldn't get file path");
            txc.send(Event::LoadROM(filename)).unwrap();
        }
    });
}

fn show_cart_picker(event_tx: kanal::Sender<Event>, window: &ApplicationWindow) {
    let filters = gio::ListStore::new::<gtk::FileFilter>();

    let lnx_filter = gtk::FileFilter::new();
    lnx_filter.add_suffix("lnx");
    lnx_filter.set_name(Some("lnx"));
    filters.append(&lnx_filter);

    let o_filter = gtk::FileFilter::new();
    o_filter.add_suffix("o");
    o_filter.set_name(Some("o"));
    filters.append(&o_filter);

    let filedialog = gtk::FileDialog::builder()
        .title("Load cart")
        .modal(true)
        .filters(&filters)
        .build();

    let txc = event_tx.clone();
    filedialog.open(Some(window), gio::Cancellable::NONE, move |file| {
        if let Ok(file) = file {
            let filename = file.path().expect("Couldn't get file path");
            txc.send(Event::LoadCart(filename)).unwrap();
        }
    });
}

fn show_state_picker(event_tx: kanal::Sender<Event>, window: &ApplicationWindow) {
    let filters = gio::ListStore::new::<gtk::FileFilter>();

    let sal_filter = gtk::FileFilter::new();
    sal_filter.add_suffix("sal");
    sal_filter.set_name(Some("sal"));
    filters.append(&sal_filter);

    let filedialog = gtk::FileDialog::builder()
        .title("Load state")
        .modal(true)
        .filters(&filters)
        .build();

    let txc = event_tx.clone();
    filedialog.open(Some(window), gio::Cancellable::NONE, move |file| {
        if let Ok(file) = file {
            let filename = file.path().expect("Couldn't get file path");
            txc.send(Event::LoadState(filename)).unwrap();
        }
    });
}

fn show_state_writer(event_tx: kanal::Sender<Event>, window: &ApplicationWindow) {
    let filters = gio::ListStore::new::<gtk::FileFilter>();

    let sal_filter = gtk::FileFilter::new();
    sal_filter.add_suffix("sal");
    sal_filter.set_name(Some("sal"));
    filters.append(&sal_filter);

    let filedialog = gtk::FileDialog::builder()
        .title("Save state")
        .modal(true)
        .filters(&filters)
        .build();

    let txc = event_tx.clone();
    filedialog.save(Some(window), gio::Cancellable::NONE, move |file| {
        if let Ok(file) = file {
            let filename = file.path().expect("Couldn't get file path");
            txc.send(Event::SaveState(filename)).unwrap();
        }
    });
}