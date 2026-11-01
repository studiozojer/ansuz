//! Delivered canvas events, independent of the primary-pointer recognizer.
use exact_game::*;
use serde_json::{json, Value};
pub const MAX_FINGERS: usize = 10;
pub const MAX_EVENTS: usize = 8000;
const MAX_EVENT_BYTES: usize = 900_000;
#[derive(Clone, Copy, Default, Data)]
pub struct Finger {
    pub id: u64,
    pub position: Vec2,
}
#[derive(Clone, Copy, Default, Data)]
pub struct Sample {
    pub tick: u32,
    pub ms: f64,
    pub phase: u32,
    pub id: u64,
    pub position: Vec2,
    pub delta: Vec2,
    pub buttons: u32,
}
impl Sample {
    pub fn json(&self) -> Value {
        json!([
            self.tick,
            self.ms,
            self.phase,
            self.id,
            self.position.x,
            self.position.y,
            self.delta.x,
            self.delta.y,
            self.buttons
        ])
    }
}
#[derive(Default, Data)]
pub struct Multitouch {
    pub active: Vec<Finger>,
    pub initial: Vec<Finger>,
    pub events: Vec<Sample>,
    pub dropped: u32,
    pub interrupted: bool,
    pub interrupted_at: u32,
    pub replay_cursor: u32,
    bytes: u32,
}
impl Multitouch {
    pub fn start(&mut self) {
        self.initial = self.active.clone();
        self.events.clear();
        self.dropped = 0;
        self.interrupted = false;
        self.interrupted_at = 0;
        self.replay_cursor = 0;
        self.bytes = 0;
    }
    pub fn reset_replay(&mut self) {
        self.active = self.initial.clone();
        self.replay_cursor = 0;
    }
    pub fn replay(&mut self, tick: u32) {
        while let Some(e) = self.events.get(self.replay_cursor as usize).copied() {
            if e.tick > tick {
                break;
            }
            self.apply(e);
            self.replay_cursor += 1;
        }
    }
    fn apply(&mut self, e: Sample) {
        match e.phase {
            0 => {
                self.active.retain(|p| p.id != e.id);
                if self.active.len() < MAX_FINGERS {
                    self.active.push(Finger {
                        id: e.id,
                        position: e.position,
                    });
                }
            }
            1 => {
                if let Some(p) = self.active.iter_mut().find(|p| p.id == e.id) {
                    p.position = e.position;
                }
            }
            2 if e.buttons != 0 => {}
            2 | 3 => self.active.retain(|p| p.id != e.id),
            4 => self.active.clear(),
            _ => {}
        }
    }
    pub fn deliver(&mut self, input: &Input, tick: u32, recording: bool) {
        if recording {
            self.dropped = self.dropped.saturating_add(input.pointer_events_dropped());
        }
        for event in input.pointer_events() {
            let (phase, id, position, delta, buttons, at) = match *event {
                InputEvent::Pointer {
                    id,
                    phase,
                    x,
                    y,
                    dx,
                    dy,
                    buttons,
                    at_ms,
                    ..
                } => (
                    match phase {
                        PointerPhase::Down => 0,
                        PointerPhase::Move => 1,
                        PointerPhase::Up => 2,
                        PointerPhase::Cancel => 3,
                    },
                    id,
                    Vec2::new(x, y),
                    Vec2::new(dx, dy),
                    buttons,
                    at_ms,
                ),
                InputEvent::Blur { at_ms, .. } => (4, 0, Vec2::ZERO, Vec2::ZERO, 0, at_ms),
                _ => continue,
            };
            // Hovers are not contacts and have no corresponding Down.
            if (1..=3).contains(&phase) && !self.active.iter().any(|p| p.id == id) {
                continue;
            }
            let too_many = phase == 0
                && self.active.len() >= MAX_FINGERS
                && !self.active.iter().any(|p| p.id == id);
            let e = Sample {
                tick,
                ms: at,
                phase,
                id,
                position,
                delta,
                buttons,
            };
            self.apply(e);
            if recording {
                if phase == 4 && !self.interrupted {
                    self.interrupted = true;
                    self.interrupted_at = tick;
                }
                let bytes = e.json().to_string().len() + 1;
                if !too_many
                    && self.dropped == 0
                    && self.events.len() < MAX_EVENTS
                    && self.bytes as usize + bytes <= MAX_EVENT_BYTES
                {
                    self.events.push(e);
                    self.bytes += bytes as u32;
                } else {
                    self.dropped = self.dropped.saturating_add(1);
                }
            }
        }
    }
    pub fn json_initial(&self) -> Value {
        json!(self
            .initial
            .iter()
            .map(|p| json!([p.id, p.position.x, p.position.y]))
            .collect::<Vec<_>>())
    }
}
