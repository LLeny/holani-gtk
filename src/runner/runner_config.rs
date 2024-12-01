use core::fmt;
use std::{collections::HashMap, default, path::PathBuf};
use serde::{Serialize, Deserialize};
use gtk::gdk;
use holani::cartridge::lnx_header::LNXRotation;
use strum_macros::EnumIter;

#[derive(Clone, Serialize, Deserialize, Debug, Default, Copy, EnumIter, PartialEq)]
pub(crate) enum Input {
    #[default]
    Up,
    Down,
    Left,
    Right,
    Outside,
    Inside,
    Option1,
    Option2,
    Pause,
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub(crate) enum RunnerStatus {
    Paused,
    Running,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub(crate) enum RunnerAction {
    LoadCart,
    LoadROM,
    Reset,
    LoadState(PathBuf),
    SaveState(PathBuf),
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct RunnerConfig {
    #[serde(skip)]
    cartridge: Option<PathBuf>,
    rom: Option<PathBuf>,
    button_mapping: HashMap<String, Input>,
    mute: bool,
    comlynx: bool,
    status: RunnerStatus,
    rotation: LNXRotation,
    #[serde(skip)]
    action: Option<RunnerAction>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        let mut slf = Self {
            rom: None,
            cartridge: None,
            mute: false,
            comlynx: false,
            button_mapping: HashMap::new(),
            status: RunnerStatus::Running,
            rotation: LNXRotation::None,
            action: None,
        };

        slf.set_button_mapping(gdk::Key::Up, Input::Up);
        slf.set_button_mapping(gdk::Key::Down, Input::Down);
        slf.set_button_mapping(gdk::Key::Left, Input::Left);
        slf.set_button_mapping(gdk::Key::Right, Input::Right);
        slf.set_button_mapping(gdk::Key::q, Input::Outside);
        slf.set_button_mapping(gdk::Key::w, Input::Inside);
        slf.set_button_mapping(gdk::Key::_1, Input::Option1);
        slf.set_button_mapping(gdk::Key::_2, Input::Option2);
        slf.set_button_mapping(gdk::Key::p, Input::Pause);

        slf
    }
}

impl RunnerConfig {
    pub(crate) fn rom(&self) -> &Option<PathBuf> {
        &self.rom
    }

    pub(crate) fn set_rom(&mut self, rom: PathBuf) {
        self.rom = Some(rom);
    }

    pub(crate) fn cartridge(&self) -> &Option<PathBuf> {
        &self.cartridge
    }

    pub(crate) fn set_cartridge(&mut self, cartridge: PathBuf) {
        self.cartridge = Some(cartridge);
    }

    pub(crate) fn button_mapping(&self) -> &HashMap<String, Input> {
        &self.button_mapping
    }

    pub(crate) fn set_button_mapping(&mut self, key: gdk::Key, btn: Input) {
        self.set_button_mapping_as_str(key.name().unwrap().to_string(), btn);
    }

    pub(crate) fn set_button_mapping_as_str(&mut self, key: String, btn: Input) {
        let k = key.to_lowercase();
        if let Some((to_remove, _)) = self.button_mapping.clone().iter().find(|(_, v)| **v == btn) {
            self.button_mapping.remove(to_remove);
        }
        self.button_mapping.insert(k, btn);
    }
    
    pub(crate) fn mute(&self) -> bool {
        self.mute
    }
    
    pub(crate) fn set_mute(&mut self, mute: bool) {
        self.mute = mute;
    }
    
    pub(crate) fn comlynx(&self) -> bool {
        self.comlynx
    }
    
    pub(crate) fn set_comlynx(&mut self, comlynx: bool) {
        self.comlynx = comlynx;
    }
    
    pub(crate) fn status(&self) -> RunnerStatus {
        self.status
    }
    
    pub(crate) fn set_status(&mut self, status: RunnerStatus) {
        self.status = status;
    }
    
    pub(crate) fn rotation(&self) -> LNXRotation {
        self.rotation
    }
    
    pub(crate) fn set_rotation(&mut self, rotation: LNXRotation) {
        self.rotation = rotation;
    }
    
    pub(crate) fn take_action(&mut self) -> Option<RunnerAction> {
        self.action.take()
    }
    
    pub(crate) fn set_action(&mut self, action: RunnerAction) {
        self.action = Some(action);
    }
}
