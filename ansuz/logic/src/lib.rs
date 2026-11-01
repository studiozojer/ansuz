use exact_game::*;
pub mod elements;
pub mod lab;
pub mod multitouch;
pub mod orbit;
pub mod records;
use orbit::*;
#[derive(Default, Args)]
pub struct Options {
    #[live]
    pub mode: u32,
    #[live]
    pub home: u32,
    #[live]
    pub reduced_motion: bool,
    #[live]
    pub lab: bool,
    #[live]
    pub flat: bool,
    #[live]
    pub take: String,
    #[live]
    pub take_revision: u32,
    #[live]
    pub element: u32,
}
#[derive(Clone, Copy, Resource)]
pub struct Navigation {
    pub orientation: Quat,
    pub velocity: Vec2,
    pub down: bool,
    pub pointer: u64,
    pub home: u32,
    pub mode: u32,
    pub quiet: f32,
}
impl Default for Navigation {
    fn default() -> Self {
        Self {
            orientation: Quat::from_rotation_y(0.035) * Quat::from_rotation_x(-0.023),
            velocity: Vec2::ZERO,
            down: false,
            pointer: 0,
            home: 0,
            mode: 0,
            quiet: 0.,
        }
    }
}
#[derive(Default, Data)]
struct Hud {
    cell: String,
    mode: String,
    motion: String,
    altitude: f32,
    lab_status: String,
    lab_seconds: f32,
    lab_samples: u32,
    lab_observed: u32,
    lab_contacts: u32,
    lab_delivered: u32,
    lab_recording: bool,
    lab_replaying: bool,
    lab_ready: bool,
    lab_error: String,
    lab_element: u32,
    element_phase: String,
    element_energy: f32,
    element_charge: f32,
}
fn draw(w: &mut World, n: &Navigation, previous_cell: Option<Cell>, args: &Options) {
    let flying = n.mode == 1;
    let (mut camera, globe) = poses(n.orientation, flying);
    let flat = args.lab && args.flat;
    if flat {
        camera = Transform::at(0., 0., 10.);
    }
    w.require_mut::<Camera>("camera").projection = if flat {
        Projection::Orthographic {
            height: 2.,
            integer_scale: false,
        }
    } else {
        Projection::Perspective
    };
    w.require_mut::<Visible>("sphere").0 = !flat;
    w.require_mut::<Visible>("grid").0 = !flat;
    *w.require_mut::<Transform>("camera") = camera;
    *w.require_mut::<Transform>("sphere") = globe;
    *w.require_mut::<Transform>("grid") = globe;
    let cell = target(n.orientation, flying);
    for index in 0..4 * EDGE_STEPS {
        let name = format!("focus-{index}");
        w.require_mut::<Visible>(&name).0 = cell.is_some() && !flat;
        if let Some(cell) = cell {
            let (a, b) = edge(cell, index);
            let a = globe.rotation * a;
            let b = globe.rotation * b;
            let pose = Transform {
                position: (a + b) * 0.5,
                rotation: Quat::from_rotation_arc(Vec3::Y, (b - a).normalize()),
                scale: Vec3::new(0.009, a.distance(b) + 0.005, 0.009),
            };
            if Some(cell) != previous_cell {
                // A different cell has different edges, especially across cube
                // faces. Replace its outline without morphing unrelated sides.
                w.teleport(w.named(&name).unwrap(), pose);
            } else {
                *w.require_mut::<Transform>(&name) = pose;
            }
        }
    }
    let lab = w.resource::<lab::Lab>();
    let lab_status = if lab.recording {
        "RECORDING"
    } else if lab.replaying {
        "REPLAYING"
    } else if lab.captured {
        "TAKE READY"
    } else {
        "READY"
    }
    .into();
    let lab_seconds = if lab.replaying {
        lab.cursor
    } else {
        lab.frames.len() as u32
    } as f32
        / 120.;
    let lab_samples = lab.frames.len() as u32;
    let lab_observed = lab.observed.len() as u32;
    let lab_contacts = lab.pointers.active.len() as u32;
    let lab_delivered = lab.pointers.events.len() as u32;
    let lab_recording = lab.recording;
    let lab_replaying = lab.replaying;
    let lab_ready = lab.captured;
    let lab_error = lab.error.clone();
    let lab_element = lab.source_element;
    let position = lab.position;
    let finger = lab.finger;
    let finger_down = lab.finger_down;
    let fingers = lab.pointers.active.clone();
    let legacy_pointer = lab.replaying && lab.format_version < 4;
    let replay_viewport = lab
        .replaying
        .then(|| lab.frames[lab.cursor.saturating_sub(1) as usize].viewport);
    drop(lab);
    if args.lab {
        w.require_mut::<Visible>("lab-object").0 = flat;
        *w.require_mut::<Transform>("lab-object") = Transform {
            position: Vec3::new(position.x, -position.y, 0.),
            scale: Vec3::new(0.22, 0.22, 0.01),
            ..Default::default()
        };
        for i in 0..multitouch::MAX_FINGERS {
            let name = format!("lab-touch-{i}");
            w.require_mut::<Visible>(&name).0 = flat && !legacy_pointer && i < fingers.len();
            if let Some(f) = fingers.get(i) {
                let viewport = replay_viewport.unwrap_or(w.resource::<LabViewport>().0);
                *w.require_mut::<Transform>(&name) = Transform {
                    position: Vec3::new(
                        (f.position.x - viewport.x * 0.5) / viewport.y.max(1.) * 2.,
                        -(f.position.y - viewport.y * 0.5) / viewport.y.max(1.) * 2.,
                        0.03,
                    ),
                    scale: Vec3::new(0.035, 0.035, 0.01),
                    ..Default::default()
                };
            }
        }
        // Legacy recordings retain their original primary-pointer marker.
        w.require_mut::<Visible>("lab-finger").0 = flat && finger_down && legacy_pointer;
        let viewport = replay_viewport.unwrap_or(w.resource::<LabViewport>().0);
        *w.require_mut::<Transform>("lab-finger") = Transform {
            position: Vec3::new(
                (finger.x - viewport.x * 0.5) / viewport.y.max(1.) * 2.,
                -(finger.y - viewport.y * 0.5) / viewport.y.max(1.) * 2.,
                0.02,
            ),
            scale: Vec3::new(0.028, 0.028, 0.01),
            ..Default::default()
        };
    }
    let e = w.resource::<elements::Elements>();
    let element_phase = e.caption().to_string();
    let element_energy = e.energy;
    let element_charge = e.charge;
    if args.lab && args.element > 0 {
        let object = e.object - Vec2::Y * if e.kind == 3 { e.energy * 0.34 } else { 0. };
        let hide = e.kind == 2 && e.energy > 0.005;
        let color = if e.kind == 1 && e.energy > 0.01 {
            [0.48, 0.07, 0.018]
        } else {
            [0.38 + e.charge * 0.25; 3]
        };
        let pixels = e.pixels();
        drop(e);
        w.require_mut::<Visible>("lab-object").0 = flat && !hide;
        *w.require_mut::<Transform>("lab-object") = Transform {
            position: Vec3::new(object.x, -object.y, 0.),
            scale: Vec3::new(elements::SIZE, elements::SIZE, 0.01),
            ..Default::default()
        };
        *w.require_mut::<Material>("lab-object") = Material::glow(color);
        for (i, p) in pixels.iter().enumerate() {
            let name = format!("element-pixel-{i}");
            w.require_mut::<Visible>(&name).0 = flat && p.visible;
            if p.visible {
                *w.require_mut::<Transform>(&name) = Transform {
                    position: Vec3::new(p.position.x, -p.position.y, 0.035),
                    scale: Vec3::new(p.size, p.size, 0.008),
                    ..Default::default()
                };
                *w.require_mut::<Material>(&name) = Material::glow(p.color);
            }
        }
    } else if args.lab {
        *w.require_mut::<Material>("lab-object") = Material::glow([0.45; 3]);
        for i in 0..elements::PIXELS {
            w.require_mut::<Visible>(&format!("element-pixel-{i}")).0 = false;
        }
    }
    w.publish_record(&Hud {
        cell: cell.map_or_else(
            || "—".into(),
            |c| {
                format!(
                    "{} / {:02} · {:02}",
                    ["+Z", "+X", "−Z", "−X", "+Y", "−Y"][c.face as usize],
                    c.u + 1,
                    c.v + 1
                )
            },
        ),
        mode: if flying {
            "Camera travelling · sphere fixed"
        } else {
            "Sphere turning · camera fixed"
        }
        .into(),
        motion: if n.down {
            "IN HAND"
        } else if n.velocity.length() > 0.001 {
            "DRIFTING"
        } else {
            "AT REST"
        }
        .into(),
        altitude: ALTITUDE,
        lab_status,
        lab_seconds,
        lab_samples,
        lab_observed,
        lab_contacts,
        lab_delivered,
        lab_recording,
        lab_replaying,
        lab_ready,
        lab_error,
        lab_element,
        element_phase,
        element_energy,
        element_charge,
    });
}
#[derive(Default, Resource)]
pub struct LabViewport(pub Vec2);
pub struct Ansuz;
impl Game for Ansuz {
    const ID: &'static str = "ansuz";
    const HZ: u32 = 120;
    type Args = Options;
    fn render_scale(_: &Options) -> f32 {
        // Fine wire lines need native pixels rather than an upscaled raster.
        1.
    }
    fn register(w: &mut World, _: &std::collections::BTreeMap<&str, Value>) {
        w.register_resource::<Navigation>();
        w.register_resource::<lab::Lab>();
        w.register_resource::<LabViewport>();
        w.register_resource::<elements::Elements>();
    }
    fn actions() -> Actions {
        Actions::new()
            .stick("move", Stick::wasd().or_arrows())
            .button("orbit-contact", &["MouseLeft"])
            .button_touch("orbit-contact", Region::All)
            .button("recenter", &["Home"])
            .button("west", &[])
            .button("east", &[])
            .button("north", &[])
            .button("south", &[])
    }
    fn setup(w: &mut World, args: &Options) {
        let n = Navigation {
            mode: args.mode,
            home: args.home,
            ..Default::default()
        };
        w.insert_resource(n);
        w.insert_resource(lab::Lab::default());
        w.insert_resource(LabViewport::default());
        let mut elements = elements::Elements::default();
        elements.reset(args.element);
        w.insert_resource(elements);
        w.insert_resource(Environment {
            background: Some(Color::hex("#0b0b0b")),
            ambient: 0.,
            sun_disc: 0.,
            fog: None,
            bloom: None,
            ..Default::default()
        });
        let (shell, grid) = geometry();
        let shell = w
            .generated("orbital-shell.model", shell)
            .expect("valid spherical shell");
        let grid = w
            .generated("orbital-grid.model", grid)
            .expect("valid spherical grid");
        w.spawn_named(
            "sphere",
            (
                Transform::default(),
                shell,
                Material::glow([0.009; 3]),
                Visible(true),
            ),
        );
        w.spawn_named(
            "grid",
            (
                Transform::default(),
                grid,
                Material::glow([0.19; 3]),
                Visible(true),
            ),
        );
        w.spawn_named(
            "camera",
            (
                base_camera(),
                Camera {
                    near: 0.02,
                    far: 60.,
                    ..Default::default()
                },
            ),
        );
        for i in 0..4 * EDGE_STEPS {
            w.spawn_named(
                &format!("focus-{i}"),
                (
                    Transform::default(),
                    Mesh::cube(1.),
                    Material::glow([0.9; 3]),
                    Visible(true),
                ),
            );
        }
        if args.lab {
            for i in 0..elements::PIXELS {
                w.spawn_named(
                    &format!("element-pixel-{i}"),
                    (
                        Transform::default(),
                        Mesh::cube(1.),
                        Material::glow([0.3; 3]),
                        Visible(false),
                    ),
                );
            }
            for i in 0..multitouch::MAX_FINGERS {
                w.spawn_named(
                    &format!("lab-touch-{i}"),
                    (
                        Transform::default(),
                        Mesh::cube(1.),
                        Material::glow([0.95; 3]),
                        Visible(false),
                    ),
                );
            }
            for (name, color) in [("lab-object", 0.45), ("lab-finger", 0.95)] {
                w.spawn_named(
                    name,
                    (
                        Transform::default(),
                        Mesh::cube(1.),
                        Material::glow([color; 3]),
                        Visible(false),
                    ),
                );
            }
        }
        draw(w, &n, None, args);
    }
    fn tick(w: &mut World, input: &Input, args: &Options) {
        let mut n = *w.resource::<Navigation>();
        let previous_cell = target(n.orientation, n.mode == 1);
        let dt = w.dt();
        if n.home != args.home || input.pressed("recenter") {
            n = Navigation {
                home: args.home,
                mode: args.mode,
                ..Default::default()
            };
        }
        if n.mode != args.mode {
            n.mode = args.mode;
            n.velocity = Vec2::ZERO;
        }
        let mut lab = std::mem::take(&mut *w.resource_mut::<lab::Lab>());
        let mut elements = std::mem::take(&mut *w.resource_mut::<elements::Elements>());
        if elements.kind != args.element {
            elements.reset(args.element);
        }
        w.resource_mut::<LabViewport>().0 = input.viewport();
        if args.lab {
            lab.pointers
                .deliver(input, lab.frames.len() as u32, lab.recording);
            if lab.loaded_revision != args.take_revision {
                if let Err(e) = lab.load(&args.take) {
                    lab.error = e;
                }
                lab.loaded_revision = args.take_revision;
            }
            for message in input.messages() {
                if let Some(host) = message
                    .strip_prefix("lab:record|")
                    .and_then(|s| s.parse::<f64>().ok())
                    .filter(|n| n.is_finite())
                {
                    if args.element > 0 {
                        elements.reset(args.element);
                    }
                    lab.start(n, input.viewport(), args, host);
                } else if message == "lab:stop" {
                    if lab.recording {
                        w.emit(lab.finish());
                    }
                    lab.replaying = false;
                    n.velocity = Vec2::ZERO;
                    n.down = false;
                } else if message == "lab:replay" {
                    if args.element != lab.source_element {
                        lab.error = format!(
                            "Choose {} to replay this take.",
                            elements::name(lab.source_element)
                        );
                    } else {
                        elements.reset(lab.source_element);
                        lab.replay(&mut n);
                    }
                    n.mode = args.mode;
                } else if message == "lab:forget" {
                    lab = lab::Lab::default();
                    elements.reset(args.element);
                    n = Navigation {
                        mode: args.mode,
                        home: args.home,
                        ..Default::default()
                    };
                } else if message == "lab:reset" {
                    if lab.recording {
                        w.emit(lab.finish());
                    }
                    lab.replaying = false;
                    lab.position = Vec2::ZERO;
                    elements.reset(args.element);
                    n = Navigation {
                        mode: args.mode,
                        home: args.home,
                        ..Default::default()
                    };
                } else if let Some(text) = message.strip_prefix("lab:load|") {
                    if let Err(e) = lab.load(text) {
                        lab.error = e;
                    }
                } else {
                    lab.observe(message);
                }
            }
        }
        let live = lab::Contact::read(input);
        // A live contact interrupts a replay; it never combines with replay input.
        if lab.replaying
            && (live.began
                || input.pointer_events().iter().any(|e| {
                    matches!(
                        e,
                        InputEvent::Pointer {
                            phase: PointerPhase::Down,
                            ..
                        } | InputEvent::Blur { .. }
                    )
                }))
        {
            lab.replaying = false;
            lab.pointers.active.clear();
            lab.pointers.deliver(input, 0, false);
            n.velocity = Vec2::ZERO;
            n.down = false;
        }
        let replaying = lab.replaying;
        let contact = if replaying {
            lab.frames[lab.cursor as usize].contact
        } else {
            live
        };
        let viewport = if replaying {
            lab.frames[lab.cursor as usize].viewport
        } else {
            input.viewport()
        };
        let reduced = if replaying {
            lab.frames[lab.cursor as usize].reduced_motion
        } else {
            args.reduced_motion
        };
        let mut movement = advance_motion(&mut n, contact, dt, viewport, reduced);
        let keys = input.stick("move");
        if !replaying {
            movement += Vec2::new(-keys.x, keys.y) * dt * 0.24;
        }
        for (name, delta) in [
            ("west", Vec2::new(0.035, 0.)),
            ("east", Vec2::new(-0.035, 0.)),
            ("north", Vec2::new(0., 0.035)),
            ("south", Vec2::new(0., -0.035)),
        ] {
            if input.pressed(name) {
                movement += delta;
                n.velocity = Vec2::ZERO;
            }
        }
        n.orientation = turn(n.orientation, movement);
        if args.lab {
            elements.earth_legacy = replaying && lab.model_revision == 1;
            elements.step(&contact, viewport, dt, reduced);
            lab.position += movement * (2. / 0.45);
            if replaying {
                if lab.format_version >= 4 {
                    lab.pointers.replay(lab.cursor);
                }
                lab.finger = contact.position;
                lab.finger_down = contact.down;
                lab.cursor += 1;
                if lab.cursor as usize >= lab.frames.len() {
                    lab.replaying = false;
                    lab.finger_down = false;
                    lab.pointers.active.clear();
                    n.velocity = Vec2::ZERO;
                    n.down = false;
                }
            } else if lab.recording {
                lab.frames.push(lab::Frame {
                    contact,
                    viewport,
                    reduced_motion: reduced,
                    orientation: n.orientation,
                    position: lab.position,
                    effect_energy: elements.energy,
                    effect_phase: elements.phase,
                    effect_position: elements.object,
                });
                // Focus can leave the canvas on the Stop button's down, before
                // its press command arrives. Keep that target alive for the handoff.
                let interrupted = lab.pointers.interrupted
                    && lab.frames.len() as u32 >= lab.pointers.interrupted_at + 30;
                if lab.frames.len() >= lab::MAX_FRAMES
                    || interrupted
                    || lab.pointers.dropped > 0
                {
                    lab.stop_reason = if lab.pointers.dropped > 0 {
                        "input_limit"
                    } else if interrupted {
                        "interrupted"
                    } else {
                        "duration_limit"
                    }
                    .into();
                    w.emit(lab.finish());
                }
            }
            if lab.recording || lab.replaying {
                w.busy("gesture lab");
            }
        }
        if args.lab && args.element > 0 && (elements.grabbed || elements.energy > 0.001) {
            w.busy("element response");
        }
        *w.resource_mut::<elements::Elements>() = elements;
        *w.resource_mut::<lab::Lab>() = lab;
        if n.velocity.length() > 0.001 {
            w.busy("orbital drift");
        }
        *w.resource_mut::<Navigation>() = n;
        draw(w, &n, previous_cell, args);
    }
}

pub fn advance_motion(
    n: &mut Navigation,
    c: lab::Contact,
    dt: f32,
    viewport: Vec2,
    reduced_motion: bool,
) -> Vec2 {
    let began = c.began;
    let ended = c.ended;
    let down = c.present && c.down && c.has_origin;
    let released_primary = ended && (began || n.down) && c.present && c.has_origin;
    let mut movement = Vec2::ZERO;
    // Edges survive a complete gesture between ticks. Catch a new contact
    // even when it has already lifted (or been cancelled) by this update.
    if began {
        n.velocity = Vec2::ZERO;
        n.quiet = 0.;
    }
    if down || released_primary {
        if !n.down || n.pointer != c.id {
            n.velocity = Vec2::ZERO;
            n.quiet = 0.;
        }
        n.quiet += dt;
        // Touch Down is independent of mouse buttons. A stationary finger
        // must stop drift immediately, without needing recognition slop.
        // On a new contact, use its own travel, excluding earlier hover
        // motion in the same tick. Include the terminal sample before Up.
        let delta = if began {
            c.position - c.origin
        } else {
            c.delta
        };
        if delta.length_squared() > 0. {
            let sensitivity = 0.45 / viewport.y.max(240.);
            movement = delta * sensitivity;
            n.velocity = (movement / n.quiet.max(dt)).clamp_length_max(0.7);
            n.quiet = 0.;
        } else if n.quiet > 0.075 {
            n.velocity = Vec2::ZERO;
        }
        n.pointer = c.id;
    } else if (n.down || ended) && (!c.present || !c.has_origin) {
        n.velocity = Vec2::ZERO;
    } else if !reduced_motion {
        let decay = math::exp(-5.5 * dt);
        movement = n.velocity * ((1. - decay) / 5.5);
        n.velocity *= decay;
        if n.velocity.length() < 0.001 {
            n.velocity = Vec2::ZERO;
        }
    }
    if reduced_motion {
        n.velocity = Vec2::ZERO;
    }
    n.down = down;
    movement
}
