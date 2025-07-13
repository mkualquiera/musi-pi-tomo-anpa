use std::{
    cell::RefCell,
    future::IntoFuture,
    rc::Rc,
    sync::{Arc, Mutex},
};

use log::error;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::{ArrayBuffer, Uint8Array},
    AudioBuffer, AudioContext, AudioContextState,
};

enum LoadState {
    Loading,
    Done(AudioBuffer),
    Failed,
}

enum LoadableAudio {
    Loading(Rc<RefCell<LoadState>>),
    Loaded(AudioBuffer),
    Dummy,
}

pub struct AudioSystem {
    audio_context: Option<AudioContext>,
    audio_buffers: Vec<LoadableAudio>,
}

pub struct AudioHandle {
    index: usize,
}

impl AudioSystem {
    pub fn new() -> Self {
        Self {
            audio_context: AudioContext::new().ok(),
            audio_buffers: Vec::new(),
        }
    }

    pub fn on_user_interaction(&mut self) {
        if let Some(audio_context) = &self.audio_context {
            if audio_context.state() == AudioContextState::Suspended {
                let _ = audio_context.resume();
            }
        }
    }

    pub fn load_buffer(&mut self, bytes: &[u8]) -> AudioHandle {
        let handle = AudioHandle {
            index: self.audio_buffers.len(),
        };
        if let Some(audio_context) = &self.audio_context {
            let array_buffer = ArrayBuffer::new(bytes.len() as u32);
            let uint8_array = Uint8Array::new(&array_buffer);
            uint8_array.copy_from(bytes);

            let future = JsFuture::from(audio_context.decode_audio_data(&array_buffer).unwrap());

            let entry = Rc::new(RefCell::new(LoadState::Loading));

            let entry_clone = entry.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match future.await {
                    Ok(decoded) => {
                        // Try to cast the decoded data to AudioBuffer
                        match decoded.dyn_into::<AudioBuffer>() {
                            Ok(audio_buffer) => {
                                *entry_clone.borrow_mut() = LoadState::Done(audio_buffer);
                            }
                            Err(err) => {
                                error!("Failed to decode audio data: {:?}", err);
                                *entry_clone.borrow_mut() = LoadState::Failed;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed to decode audio data: {:?}", err);
                        *entry_clone.borrow_mut() = LoadState::Failed;
                    }
                }
            });

            self.audio_buffers.push(LoadableAudio::Loading(entry));
        } else {
            log::error!("Audio context is not initialized");
            self.audio_buffers.push(LoadableAudio::Dummy);
        }
        handle
    }

    pub fn play(&mut self, handle: &AudioHandle, speed: f32) {
        // If it's dummy, do nothing
        // If it's loading and failed, convert to dummy
        // If it's loading and done, convert to loaded and call play again
        // If it's loaded, play the audio

        enum QueryResult {
            IntoLoaded,
            IntoDummy,
            Noop,
            DoPlay,
        }

        let result = match &self.audio_buffers[handle.index] {
            LoadableAudio::Dummy => {
                log::warn!("Attempted to play a dummy audio handle");
                QueryResult::Noop
            }
            LoadableAudio::Loading(state) => {
                let state = state.borrow();
                match &*state {
                    LoadState::Loading => {
                        log::warn!("Audio is still loading, cannot play yet");
                        QueryResult::Noop
                    }
                    LoadState::Done(audio_buffer) => QueryResult::IntoLoaded,
                    LoadState::Failed => {
                        log::error!("Failed to load audio, converting to dummy");
                        QueryResult::IntoDummy
                    }
                }
            }
            LoadableAudio::Loaded(audio_buffer) => QueryResult::DoPlay,
        };
        match result {
            QueryResult::IntoLoaded => {
                let audio_buffer = match &self.audio_buffers[handle.index] {
                    LoadableAudio::Loading(state) => {
                        let state = state.borrow();
                        if let LoadState::Done(audio_buffer) = &*state {
                            audio_buffer.clone()
                        } else {
                            log::error!("Expected audio to be loaded, but it was not");
                            return;
                        }
                    }
                    _ => unreachable!(),
                };
                self.audio_buffers[handle.index] = LoadableAudio::Loaded(audio_buffer);
                self.play(handle, speed); // Call play again with the loaded audio
            }
            QueryResult::IntoDummy => {
                self.audio_buffers[handle.index] = LoadableAudio::Dummy;
            }
            QueryResult::Noop => {}
            QueryResult::DoPlay => {
                if let LoadableAudio::Loaded(audio_buffer) = &self.audio_buffers[handle.index] {
                    if let Some(audio_context) = &self.audio_context {
                        let source = audio_context.create_buffer_source().unwrap();
                        source.set_buffer(Some(audio_buffer));
                        source.playback_rate().set_value(speed); // Set playback speed
                        source
                            .connect_with_audio_node(&audio_context.destination())
                            .unwrap();
                        source.start().unwrap();
                    } else {
                        log::error!("Audio context is not initialized");
                    }
                }
            }
        }
    }
}
