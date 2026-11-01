//! Bounded, portable gesture takes. The observer and delivered-input tracks
//! are intentionally separate: replaying engine samples cannot prove UIKit delivery.
use crate::{Navigation, Options};
use exact_game::*;
use serde_json::{json, Value as Json};
pub const MAX_FRAMES: usize = 2400; // 20 seconds at 120 Hz
pub const MAX_OBSERVED: usize = 4800;
#[derive(Clone, Copy, Default, Data)]
pub struct Contact {
    pub present: bool,
    pub id: u64,
    pub position: Vec2,
    pub origin: Vec2,
    pub has_origin: bool,
    pub delta: Vec2,
    pub down: bool,
    pub began: bool,
    pub ended: bool,
}
impl Contact {
    pub fn read(input: &Input) -> Self {
        let p = input.pointer();
        Self {
            present: p.is_some(),
            id: p.map_or(0, |p| p.id),
            position: p.map_or(Vec2::ZERO, |p| p.position),
            origin: p.and_then(|p| p.press_origin).unwrap_or(Vec2::ZERO),
            has_origin: p.is_some_and(|p| p.press_origin.is_some()),
            delta: p.map_or(Vec2::ZERO, |p| p.delta),
            down: p.is_some_and(|p| p.down),
            began: input.pressed("orbit-contact"),
            ended: input.released("orbit-contact"),
        }
    }
    fn flags(self) -> u32 {
        u32::from(self.present)
            | (u32::from(self.has_origin) << 1)
            | (u32::from(self.began) << 2)
            | (u32::from(self.ended) << 3)
    }
}
#[derive(Clone, Copy, Default, Data)]
pub struct Frame {
    pub contact: Contact,
    pub viewport: Vec2,
    pub reduced_motion: bool,
    pub effect_energy: f32,
    pub effect_phase: u32,
    pub effect_position: Vec2,
    pub orientation: Quat,
    pub position: Vec2,
}
#[derive(Clone, Copy, Default, Data)]
pub struct Observation {
    pub tick: u32,
    pub ms: f64,
    pub phase: u32,
    pub id: u64,
    pub position: Vec2,
    pub buttons: u32,
}
#[derive(Default, Resource)]
pub struct Lab {
    pub loaded_revision: u32,
    pub recording: bool,
    pub replaying: bool,
    pub cursor: u32,
    pub frames: Vec<Frame>,
    pub observed: Vec<Observation>,
    pub initial: Navigation,
    pub initial_position: Vec2,
    pub position: Vec2,
    pub viewport: Vec2,
    pub reduced_motion: bool,
    pub source_mode: u32,
    pub source_element: u32,
    pub host_start: f64,
    pub error: String,
    pub captured: bool,
    pub finger: Vec2,
    pub finger_down: bool,
}
impl Lab {
    pub fn start(&mut self, n: Navigation, viewport: Vec2, args: &Options, host: f64) {
        self.initial = n;
        self.initial_position = self.position;
        self.viewport = viewport;
        self.reduced_motion = args.reduced_motion;
        self.source_mode = args.mode;
        self.source_element = args.element;
        self.host_start = host;
        self.frames.clear();
        self.observed.clear();
        self.cursor = 0;
        self.recording = true;
        self.replaying = false;
        self.captured = false;
        self.error.clear();
    }
    pub fn observe(&mut self, message: &str) {
        let parts: Vec<_> = message.split('|').collect();
        if parts.len() != 7 || parts[0] != "finger" {
            return;
        }
        let phase = match parts[1] {
            "down" => 0,
            "move" => 1,
            "up" => 2,
            _ => return,
        };
        let parsed = (
            parts[2].parse::<u64>(),
            parts[3].parse::<f32>(),
            parts[4].parse::<f32>(),
            parts[5].parse::<u32>(),
            parts[6].parse::<f64>(),
        );
        let (Ok(id), Ok(x), Ok(y), Ok(buttons), Ok(at)) = parsed else {
            return;
        };
        if !x.is_finite() || !y.is_finite() || !at.is_finite() {
            return;
        }
        self.finger = Vec2::new(x, y);
        self.finger_down = phase != 2 && buttons != 0;
        if self.recording && self.observed.len() < MAX_OBSERVED {
            self.observed.push(Observation {
                tick: self.frames.len() as u32,
                ms: (at - self.host_start).max(0.),
                phase,
                id,
                position: self.finger,
                buttons,
            });
        }
    }
    pub fn finish(&mut self) -> String {
        self.recording = false;
        self.captured = true;
        self.export()
    }
    pub fn replay(&mut self, n: &mut Navigation) {
        if self.frames.is_empty() {
            self.error = "Record a take first.".into();
            return;
        }
        self.recording = false;
        self.replaying = true;
        self.cursor = 0;
        self.position = self.initial_position;
        *n = self.initial;
        self.error.clear();
    }
    pub fn export(&self) -> String {
        let frames: Vec<_> = self
            .frames
            .iter()
            .map(|f| {
                let c = f.contact;
                json!([
                    c.flags(),
                    c.id,
                    c.down,
                    c.position.x,
                    c.position.y,
                    c.delta.x,
                    c.delta.y,
                    c.origin.x,
                    c.origin.y,
                    f.orientation.x,
                    f.orientation.y,
                    f.orientation.z,
                    f.orientation.w,
                    f.position.x,
                    f.position.y,
                    f.viewport.x,
                    f.viewport.y,
                    f.reduced_motion,
                    f.effect_energy,
                    f.effect_phase,
                    f.effect_position.x,
                    f.effect_position.y
                ])
            })
            .collect();
        let observed: Vec<_> = self
            .observed
            .iter()
            .map(|s| {
                json!([
                    s.tick,
                    s.ms,
                    s.phase,
                    s.id,
                    s.position.x,
                    s.position.y,
                    s.buttons
                ])
            })
            .collect();
        json!({"schema":"ansuz.gesture-take","version":3,"element":self.source_element,"element_model":"ansuz.elements/1","hz":120,"viewport":self.viewport.to_array(),"mode":self.source_mode,
            "reduced_motion":self.reduced_motion,"input_layer":"engine tick snapshots; observer events before gesture recognition",
            "limitations":"Primary-pointer replay. Observer up includes cancellation. Not raw digitizer samples or displayed-frame measurements.",
            "frame_columns":["flags","pointer_id","down","x","y","dx","dy","origin_x","origin_y","qx","qy","qz","qw","object_x","object_y","viewport_w","viewport_h","reduced_motion","effect_energy","effect_phase","effect_x","effect_y"],
            "observer_columns":["tick","host_ms_from_record","phase_0_down_1_move_2_up","pointer_id","x","y","buttons"],
            "initial":{"orientation":self.initial.orientation.to_array(),"velocity":self.initial.velocity.to_array(),"down":self.initial.down,
                "pointer":self.initial.pointer,"quiet":self.initial.quiet,"position":self.initial_position.to_array()},
            "frames":frames,"observed":observed}).to_string()
    }
    pub fn load(&mut self, text: &str) -> Result<(), String> {
        if text.len() > 2 * 1024 * 1024 {
            return Err("Take exceeds 2 MiB.".into());
        }
        let j: Json = serde_json::from_str(text).map_err(|_| "Invalid take JSON.")?;
        if j["schema"] != "ansuz.gesture-take"
            || !matches!(j["version"].as_u64(), Some(1 | 2 | 3))
            || j["hz"] != 120
        {
            return Err("Unsupported gesture take.".into());
        }
        let source_element = if j["version"] == 3 {
            let element = j["element"]
                .as_u64()
                .filter(|v| *v <= 4)
                .ok_or("Invalid element.")? as u32;
            if j["element_model"] != "ansuz.elements/1" {
                return Err("Unsupported element model.".into());
            }
            element
        } else {
            0
        };
        let array = |v: &Json, n: usize| -> Result<Vec<f32>, String> {
            let a = v
                .as_array()
                .filter(|a| a.len() == n)
                .ok_or("Invalid numeric sample.")?;
            a.iter()
                .map(|v| {
                    v.as_f64()
                        .filter(|f| f.is_finite() && f.abs() < 1e7)
                        .map(|v| v as f32)
                        .ok_or("Invalid numeric sample.".into())
                })
                .collect()
        };
        let view = array(&j["viewport"], 2)?;
        if view[0] < 1. || view[1] < 1. {
            return Err("Invalid recorded viewport.".into());
        }
        let q = array(&j["initial"]["orientation"], 4)?;
        let q = Quat::from_xyzw(q[0], q[1], q[2], q[3]);
        if (q.length() - 1.).abs() > 0.01 {
            return Err("Invalid initial orientation.".into());
        }
        let v = array(&j["initial"]["velocity"], 2)?;
        let pos = array(&j["initial"]["position"], 2)?;
        let mode = j["mode"]
            .as_u64()
            .filter(|m| *m < 2)
            .ok_or("Invalid mode.")? as u32;
        let quiet = j["initial"]["quiet"]
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0. && *v < 1e7)
            .ok_or("Invalid quiet time.")? as f32;
        let reduced = j["reduced_motion"]
            .as_bool()
            .ok_or("Missing motion preference.")?;
        let mut frames = Vec::new();
        for r in j["frames"]
            .as_array()
            .filter(|a| !a.is_empty() && a.len() <= MAX_FRAMES)
            .ok_or("Invalid take length.")?
        {
            let a = r
                .as_array()
                .filter(|a| {
                    (j["version"] == 3 && a.len() == 22)
                        || (j["version"] != 3 && a.len() == 18)
                        || (j["version"] == 1 && a.len() == 15)
                })
                .ok_or("Invalid frame.")?;
            let flags = a[0].as_u64().filter(|v| *v < 16).ok_or("Invalid flags.")?;
            let id = a[1].as_u64().ok_or("Invalid pointer ID.")?;
            let mut numbers = array(
                &Json::Array(a[3..a.len().min(17)].to_vec()),
                if a.len() == 15 { 12 } else { 14 },
            )?;
            if a.len() == 15 {
                numbers.extend([view[0], view[1]]);
            }
            if numbers[12] < 1. || numbers[13] < 1. {
                return Err("Invalid sample viewport.".into());
            }
            let effects = if a.len() == 22 {
                array(&Json::Array(a[18..22].to_vec()), 4)?
            } else {
                vec![0.; 4]
            };
            if !(0. ..=1.).contains(&effects[0])
                || !(0. ..=4.).contains(&effects[1])
                || effects[1].fract() != 0.
            {
                return Err("Invalid effect response.".into());
            }
            let f = Frame {
                effect_energy: effects[0],
                effect_phase: effects[1] as u32,
                effect_position: Vec2::new(effects[2], effects[3]),
                viewport: Vec2::new(numbers[12], numbers[13]),
                reduced_motion: if a.len() == 15 {
                    reduced
                } else {
                    a[17].as_bool().ok_or("Invalid sample motion preference.")?
                },
                contact: Contact {
                    present: flags & 1 != 0,
                    has_origin: flags & 2 != 0,
                    began: flags & 4 != 0,
                    ended: flags & 8 != 0,
                    id,
                    down: a[2].as_bool().ok_or("Invalid contact.")?,
                    position: Vec2::new(numbers[0], numbers[1]),
                    delta: Vec2::new(numbers[2], numbers[3]),
                    origin: Vec2::new(numbers[4], numbers[5]),
                },
                orientation: Quat::from_xyzw(numbers[6], numbers[7], numbers[8], numbers[9]),
                position: Vec2::new(numbers[10], numbers[11]),
            };
            if (f.orientation.length() - 1.).abs() > 0.01 {
                return Err("Invalid sample orientation.".into());
            }
            frames.push(f);
        }
        let mut observed = Vec::new();
        for r in j["observed"]
            .as_array()
            .filter(|a| a.len() <= MAX_OBSERVED)
            .ok_or("Invalid observer track.")?
        {
            let a = array(r, 7)?;
            if a[0] < 0. || a[0] as usize > frames.len() || a[1] < 0. || !(0. ..=2.).contains(&a[2])
            {
                return Err("Invalid observer event.".into());
            }
            observed.push(Observation {
                tick: a[0] as u32,
                ms: a[1] as f64,
                phase: a[2] as u32,
                id: a[3] as u64,
                position: Vec2::new(a[4], a[5]),
                buttons: a[6] as u32,
            });
        }
        let initial_down = j["initial"]["down"]
            .as_bool()
            .ok_or("Invalid initial contact.")?;
        let initial_pointer = j["initial"]["pointer"]
            .as_u64()
            .ok_or("Invalid initial pointer.")?;
        self.frames = frames;
        self.observed = observed;
        self.viewport = Vec2::new(view[0], view[1]);
        self.source_mode = mode;
        self.source_element = source_element;
        self.reduced_motion = reduced;
        self.initial = Navigation {
            orientation: q,
            velocity: Vec2::new(v[0], v[1]),
            down: initial_down,
            pointer: initial_pointer,
            quiet,
            mode,
            ..Default::default()
        };
        self.initial_position = Vec2::new(pos[0], pos[1]);
        self.recording = false;
        self.replaying = false;
        self.captured = true;
        self.cursor = 0;
        self.error.clear();
        Ok(())
    }
}
