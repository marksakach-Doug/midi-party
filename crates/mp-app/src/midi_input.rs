//! ALSA sequencer MIDI input. A worker thread reads device events and sends
//! compact messages to the UI; callbacks never mutate the song or audio engine.

use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub enum MidiInputEvent {
    NoteOn {
        key: u8,
        velocity: u8,
        at: Instant,
    },
    NoteOff {
        key: u8,
        at: Instant,
    },
    ControlChange {
        controller: u8,
        value: u8,
        at: Instant,
    },
    PitchBend {
        value: i32,
        at: Instant,
    },
}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug)]
struct Port {
    addr: alsa::seq::Addr,
    name: String,
}

#[cfg(target_os = "linux")]
pub fn list_ports() -> Vec<String> {
    use alsa::seq::{ClientIter, PortCap, PortIter, Seq};
    let Ok(seq) = Seq::open(None, None, true) else {
        return Vec::new();
    };
    ClientIter::new(&seq)
        .flat_map(|client| {
            PortIter::new(&seq, client.get_client())
                .filter_map(|p| {
                    let caps = p.get_capability();
                    if !caps.contains(PortCap::READ | PortCap::SUBS_READ) {
                        return None;
                    }
                    Some(format!(
                        "{}: {}",
                        client.get_name().ok()?,
                        p.get_name().ok()?
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(not(target_os = "linux"))]
pub fn list_ports() -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "linux")]
fn ports() -> Result<Vec<Port>, String> {
    use alsa::seq::{ClientIter, PortCap, PortIter, Seq};
    let seq = Seq::open(None, None, true).map_err(|e| e.to_string())?;
    Ok(ClientIter::new(&seq)
        .flat_map(|client| {
            let client_name = client.get_name().unwrap_or("MIDI").to_string();
            PortIter::new(&seq, client.get_client())
                .filter_map(move |p| {
                    let caps = p.get_capability();
                    if !caps.contains(PortCap::READ | PortCap::SUBS_READ) {
                        return None;
                    }
                    Some(Port {
                        addr: p.addr(),
                        name: format!("{client_name}: {}", p.get_name().ok()?),
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect())
}

pub struct MidiInputState {
    rx: Receiver<MidiInputEvent>,
    #[cfg(target_os = "linux")]
    worker: Option<(
        std::sync::Arc<std::sync::atomic::AtomicBool>,
        std::thread::JoinHandle<()>,
    )>,
    pub connected_name: Option<String>,
}

impl MidiInputState {
    pub fn new() -> Self {
        let (_tx, rx) = channel();
        Self {
            rx,
            #[cfg(target_os = "linux")]
            worker: None,
            connected_name: None,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn connect(&mut self, index: usize, repaint: egui::Context) -> Result<(), String> {
        use alsa::seq::{Addr, EvCtrl, EvNote, EventType, PortCap, PortSubscribe, PortType, Seq};
        use alsa::Direction;
        self.disconnect();
        let available = ports()?;
        if available.is_empty() {
            return Err(
                "No ALSA MIDI inputs found; check the USB connection and ALSA sequencer".into(),
            );
        }
        let port = available
            .get(index)
            .cloned()
            .ok_or_else(|| "MIDI input disappeared; refresh the device list".to_string())?;
        let seq = Seq::open(None, Some(Direction::Capture), true).map_err(|e| e.to_string())?;
        seq.set_client_name(&std::ffi::CString::new("midi-party").unwrap())
            .map_err(|e| e.to_string())?;
        let local_port = seq
            .create_simple_port(
                &std::ffi::CString::new("input").unwrap(),
                PortCap::WRITE | PortCap::SUBS_WRITE,
                PortType::MIDI_GENERIC | PortType::APPLICATION,
            )
            .map_err(|e| e.to_string())?;
        let sub = PortSubscribe::empty().map_err(|e| e.to_string())?;
        sub.set_sender(port.addr);
        sub.set_dest(Addr {
            client: seq.client_id().map_err(|e| e.to_string())?,
            port: local_port,
        });
        seq.subscribe_port(&sub).map_err(|e| e.to_string())?;

        let (tx, rx) = channel();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_stop = stop.clone();
        let repaint_ctx = repaint.clone();
        let worker = std::thread::Builder::new()
            .name("midi-party-input".into())
            .spawn(move || {
                let mut input = seq.input();
                while !worker_stop.load(std::sync::atomic::Ordering::Relaxed) {
                    match input.event_input_pending(true) {
                        Ok(n) if n > 0 => {
                            let Ok(event) = input.event_input() else {
                                continue;
                            };
                            let at = Instant::now();
                            let parsed =
                                match event.get_type() {
                                    EventType::Noteon => event.get_data::<EvNote>().and_then(|n| {
                                        if n.velocity == 0 {
                                            Some(MidiInputEvent::NoteOff { key: n.note, at })
                                        } else {
                                            Some(MidiInputEvent::NoteOn {
                                                key: n.note,
                                                velocity: n.velocity,
                                                at,
                                            })
                                        }
                                    }),
                                    EventType::Noteoff => event
                                        .get_data::<EvNote>()
                                        .map(|n| MidiInputEvent::NoteOff { key: n.note, at }),
                                    EventType::Controller => event.get_data::<EvCtrl>().map(|c| {
                                        MidiInputEvent::ControlChange {
                                            controller: c.param.min(127) as u8,
                                            value: c.value.clamp(0, 127) as u8,
                                            at,
                                        }
                                    }),
                                    EventType::Pitchbend => event.get_data::<EvCtrl>().map(|c| {
                                        MidiInputEvent::PitchBend {
                                            value: c.value.clamp(-8192, 8191),
                                            at,
                                        }
                                    }),
                                    _ => None,
                                };
                            if let Some(message) = parsed {
                                // Never drop a note-on/note-off because the UI is busy
                                // for a frame. MIDI events are tiny and the receiver
                                // drains them every repaint.
                                let _ = tx.send(message);
                                repaint_ctx.request_repaint();
                            }
                        }
                        _ => std::thread::sleep(std::time::Duration::from_millis(2)),
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        self.rx = rx;
        self.worker = Some((stop, worker));
        self.connected_name = Some(port.name);
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn connect(&mut self, _index: usize, _repaint: egui::Context) -> Result<(), String> {
        Err("MIDI input is currently supported on Linux only".into())
    }

    pub fn disconnect(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some((stop, worker)) = self.worker.take() {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            let _ = worker.join();
        }
        self.connected_name = None;
        let (_tx, rx) = channel();
        self.rx = rx;
    }

    pub fn try_recv(&self) -> Option<MidiInputEvent> {
        match self.rx.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

impl Drop for MidiInputState {
    fn drop(&mut self) {
        self.disconnect();
    }
}
