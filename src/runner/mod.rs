use std::thread::JoinHandle;
use holani::cartridge::lnx_header::LNXRotation;
use log::trace;
use perframe_runner_thread::PerFrameRunnerThread;
use runner_config::RunnerConfig;
use thread_priority::*;

use crate::Event;

pub(crate) mod runner_config;
pub(crate) mod perframe_runner_thread;

pub const CRYSTAL_FREQUENCY: u32 = 16_000_000;
pub const SAMPLE_RATE: u32 = 16_000;

pub(crate) trait RunnerThread {
    fn initialize(&mut self) -> Result<(), &str>;
    fn run(&mut self);
}

pub(crate) struct Runner {
    runner_thread: Option<JoinHandle<()>>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            runner_thread: None,
        }
    }

    pub fn initialize_thread(&mut self, event_tx: kanal::Sender<Event>, config: RunnerConfig) -> (kanal::Sender<(u8, u8)>, kanal::Sender<RunnerConfig>, LNXRotation) {
        let (input_tx, input_rx) = kanal::unbounded::<(u8, u8)>();
        let (config_tx, config_rx) = kanal::unbounded::<RunnerConfig>();
        let (rotation_tx, rotation_rx) = kanal::unbounded::<LNXRotation>();

        let conf = config.clone();

        self.runner_thread = Some(
            std::thread::Builder::new()
            .name("Core".to_string())
            .spawn_with_priority(ThreadPriority::Max, move |_| {
                let mut thread: Box<dyn RunnerThread> = Box::new(PerFrameRunnerThread::new(conf, input_rx, config_rx, event_tx, rotation_tx));
                trace!("Runner started.");
                thread.initialize().unwrap_or_else(|err| {
                    println!("Error: {}", err);
                    std::process::exit(1);
                });
                thread.run();
            })
            .expect("Could not create the main core runner thread.")
        );

        let rotation = rotation_rx.recv().unwrap();
       
        (input_tx, config_tx, rotation)
    }
}
