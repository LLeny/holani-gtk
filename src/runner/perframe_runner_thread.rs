use std::{path::PathBuf, time::{Duration, Instant}};
use holani::{cartridge::lnx_header::LNXRotation, Lynx};
use log::trace;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};

use crate::Event;

use super::{runner_config::{RunnerAction, RunnerStatus}, RunnerConfig, RunnerThread, CRYSTAL_FREQUENCY, SAMPLE_RATE};
const TICKS_PER_AUDIO_SAMPLE: u64 = CRYSTAL_FREQUENCY as u64 / SAMPLE_RATE as u64;

pub(crate) struct PerFrameRunnerThread {
    lynx: Lynx,
    sound_tick: u64,
    sound_sample: Vec<i16>,
    config: RunnerConfig,
    input_rx: kanal::Receiver<(u8, u8)>,
    config_rx: kanal::Receiver<RunnerConfig>,
    event_tx: kanal::Sender<Event>,
    rotation_tx: kanal::Sender<LNXRotation>,
    frame_time: Duration,
    next_lcd_refresh: Instant,
    last_refresh_rate: f64,
    sink: Option<Sink>,
    stream: Option<OutputStream>,
}

impl PerFrameRunnerThread {
    pub(crate) fn new(
        config: RunnerConfig, 
        input_rx: kanal::Receiver<(u8, u8)>, 
        config_rx: kanal::Receiver<RunnerConfig>, 
        event_tx: kanal::Sender<Event>, 
        rotation_tx: kanal::Sender<LNXRotation>,
    ) -> Self {
        Self {
            lynx: Lynx::new(),
            config,
            input_rx,
            config_rx,
            event_tx,
            rotation_tx,
            sound_tick: 0,
            sound_sample: vec![],
            frame_time: Duration::from_millis(16),
            last_refresh_rate: 0f64,
            next_lcd_refresh: Instant::now(),
            sink: None,
            stream: None,
        }
    }

    fn sound(&mut self) {
        if self.config.mute() {
            return;
        }

        self.sound_tick += 1;

        if self.sound_tick != TICKS_PER_AUDIO_SAMPLE {
            return;
        }

        self.sound_tick = 0;
        let (l, r) = self.lynx.audio_sample();
        self.sound_sample.push(l);
        self.sound_sample.push(r);        
    }

    fn display(&mut self) {
        trace!("Display updated.");
        let _ = self.event_tx.try_send(Event::UpdateDisplay(self.lynx.screen_rgb().to_vec())).is_ok();
    }

    fn inputs(&mut self) -> bool {
        if self.input_rx.is_disconnected() {
            return true;
        } else if let Ok(Some((joy, sw))) = self.input_rx.try_recv() {
            self.lynx.set_joystick_u8(joy);
            self.lynx.set_switches_u8(sw);
        }
        false
    }

    fn config_update(&mut self)  {
        if let Ok(Some(config)) = self.config_rx.try_recv() {
            self.config = config;
        }

        if let Some(action) = self.config.take_action() {
            match action {
                RunnerAction::LoadCart => {
                    self.load_cart();
                    self.reset()
                }
                RunnerAction::LoadROM => {
                    self.load_rom();
                    self.reset();
                }
                RunnerAction::Reset => self.reset(),
                RunnerAction::LoadState(file) => self.load_state(file),
                RunnerAction::SaveState(file) => self.save_state(file),
            }
        }
    }

    fn reset(&mut self) {
        if self.config.cartridge().is_none() {
            return;
        }
        self.lynx.reset();
    }

    fn load_state(&mut self, file: PathBuf) {
        match std::fs::read(file) {
            Err(e) => println!("deserialization error: {:?}", e),
            Ok(data) => match holani::deserialize(&data, &self.lynx) {
                Err(e) => println!("deserialization error: {:?}", e),
                Ok(mut lynx) => {
                    lynx.set_comlynx_cable(&self.lynx.comlynx_cable().clone());
                    self.lynx = lynx;
                },
            }
        };
    }

    fn save_state(&mut self, file: PathBuf) {
        let size = self.lynx.serialize_size();
        let mut data: Vec<u8> = vec![0; size];
        match holani::serialize(&self.lynx, data.as_mut_slice()){
            Err(_) => panic!(),
            Ok(_)  => if std::fs::write(file, data).is_err() { panic!() }
        };
    }

    fn load_cart(&mut self) {
        if let Some(cart) = self.config.cartridge() {
            let data = std::fs::read(cart);            
            if data.is_err() {
                return;
            }
            if self.lynx.load_cart_from_slice(&data.unwrap()).is_err() {
                return;
            }
            trace!("Cart loaded.");
        } 
    }

    fn load_rom(&mut self) {
        if let Some(rom) = self.config.rom() {
            let data = std::fs::read(rom);            
            if data.is_err() {
                return;
            }
            if self.lynx.load_rom_from_slice(&data.unwrap()).is_err() {
                return;
            }
            trace!("ROM loaded.");
        }
    }
}

impl RunnerThread for PerFrameRunnerThread {
    fn initialize(&mut self) -> Result<(), &str> {
        self.load_rom();
        self.load_cart();
        self.rotation_tx.send(self.lynx.rotation()).unwrap();
        Ok(())
    }

    fn run(&mut self) {

        let mut rf: f64;

        if !self.config.mute() {
            let (stream, stream_handle) = OutputStream::try_default().unwrap();
            self.stream = Some(stream);
            self.sink = Some(Sink::try_new(&stream_handle).unwrap());
        }

        loop {
            if self.inputs() {
                return;
            }

            self.config_update();

            if self.config.cartridge().is_some() {
                while self.config.status() == RunnerStatus::Running && !self.lynx.redraw_requested() {
                    self.lynx.tick();
                    self.sound();
                }

                if !self.sound_sample.is_empty() {
                self.sink.as_mut().unwrap().append(SamplesBuffer::new(2,SAMPLE_RATE, self.sound_sample.clone()));
                self.sound_sample.clear();
                }

                rf = self.lynx.display_refresh_rate();
                if rf != self.last_refresh_rate {                
                    self.last_refresh_rate = rf;
                    self.frame_time = Duration::from_micros((1000000f64 / self.last_refresh_rate) as u64);
                    trace!("set refresh rate to {} ({:?})", rf, self.frame_time);
                } 
                self.display();
            }

            while self.next_lcd_refresh > Instant::now() {}
            self.next_lcd_refresh = Instant::now() + self.frame_time;
        }
    }
}