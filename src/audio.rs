#![allow(unused_assignments, unused_variables)]

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread;

enum AudioCmd {
    Play(Vec<u8>),
    Stop,
}

static AUDIO_TX: Mutex<Option<Sender<AudioCmd>>> = Mutex::new(None);

pub fn init() {
    let (tx, rx) = channel::<AudioCmd>();

    thread::spawn(move || {
        log::info!("Audio background thread starting...");
        // Initialize rodio output device sink. Keep _sink alive to maintain the audio device.
        let _sink: MixerDeviceSink = match DeviceSinkBuilder::open_default_sink() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to initialize rodio device sink: {:?}", e);
                return;
            }
        };

        let mut current_player: Option<Player> = None;

        while let Ok(cmd) = rx.recv() {
            match cmd {
                AudioCmd::Play(bytes) => {
                    log::info!("AudioCmd::Play received, size: {} bytes", bytes.len());
                    // Stop any currently playing sound by dropping the player
                    current_player = None;

                    let cursor = std::io::Cursor::new(bytes);
                    let source = match Decoder::try_from(cursor) {
                        Ok(s) => s,
                        Err(e) => {
                            log::error!("Failed to decode audio bytes: {:?}", e);
                            continue;
                        }
                    };

                    // Connect a Player to the device's mixer
                    let player = Player::connect_new(_sink.mixer());
                    player.append(source);
                    current_player = Some(player);
                }
                AudioCmd::Stop => {
                    log::info!("AudioCmd::Stop received");
                    current_player = None;
                }
            }
        }
    });

    if let Ok(mut guard) = AUDIO_TX.lock() {
        *guard = Some(tx);
    }
}

pub fn play_sound(bytes: Vec<u8>) {
    if let Ok(guard) = AUDIO_TX.lock() {
        if let Some(tx) = &*guard {
            let _ = tx.send(AudioCmd::Play(bytes));
        } else {
            log::info!("Audio system not initialized. Initializing background thread...");
            drop(guard);
            init();
            play_sound(bytes);
        }
    }
}

pub fn stop_sound() {
    if let Some(tx) = AUDIO_TX.lock().ok().and_then(|guard| guard.clone()) {
        let _ = tx.send(AudioCmd::Stop);
    }
}
