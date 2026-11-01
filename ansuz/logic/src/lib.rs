use exact_game::*;
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
}
fn draw(w: &mut World, n: &Navigation) {
    let flying = n.mode == 1;
    let (camera, globe) = poses(n.orientation, flying);
    *w.require_mut::<Transform>("camera") = camera;
    *w.require_mut::<Transform>("sphere") = globe;
    *w.require_mut::<Transform>("grid") = globe;
    let cell = target(n.orientation, flying);
    for index in 0..4 * EDGE_STEPS {
        let name = format!("focus-{index}");
        w.require_mut::<Visible>(&name).0 = cell.is_some();
        if let Some(cell) = cell {
            let (a, b) = edge(cell, index);
            let a = globe.rotation * a;
            let b = globe.rotation * b;
            *w.require_mut::<Transform>(&name) = Transform {
                position: (a + b) * 0.5,
                rotation: Quat::from_rotation_arc(Vec3::Y, (b - a).normalize()),
                scale: Vec3::new(0.009, a.distance(b) + 0.005, 0.009),
            };
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
    });
}
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
            (Transform::default(), shell, Material::glow([0.009; 3])),
        );
        w.spawn_named(
            "grid",
            (Transform::default(), grid, Material::glow([0.19; 3])),
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
        draw(w, &n);
    }
    fn tick(w: &mut World, input: &Input, args: &Options) {
        let mut n = *w.resource::<Navigation>();
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
        let p = input.pointer();
        let began = input.pressed("orbit-contact");
        let ended = input.released("orbit-contact");
        let down = p.is_some_and(|p| p.down && p.press_origin.is_some());
        let released_primary =
            ended && (began || n.down) && p.is_some_and(|p| p.press_origin.is_some());
        let mut movement = Vec2::ZERO;
        // Edges survive a complete gesture between ticks. Catch a new contact
        // even when it has already lifted (or been cancelled) by this update.
        if began {
            n.velocity = Vec2::ZERO;
            n.quiet = 0.;
        }
        if down || released_primary {
            let p = p.unwrap();
            if !n.down || n.pointer != p.id {
                n.velocity = Vec2::ZERO;
                n.quiet = 0.;
            }
            n.quiet += dt;
            // Touch Down is independent of mouse buttons. A stationary finger
            // must stop drift immediately, without needing recognition slop.
            // On a new contact, use its own travel, excluding earlier hover
            // motion in the same tick. Include the terminal sample before Up.
            let delta = if began {
                p.position - p.press_origin.unwrap()
            } else {
                p.delta
            };
            if delta.length_squared() > 0. {
                let sensitivity = 0.45 / input.viewport().y.max(240.);
                movement = delta * sensitivity;
                n.velocity = (movement / n.quiet.max(dt)).clamp_length_max(0.7);
                n.quiet = 0.;
            } else if n.quiet > 0.075 {
                n.velocity = Vec2::ZERO;
            }
            n.pointer = p.id;
        } else if (n.down || ended) && p.is_none_or(|p| p.press_origin.is_none()) {
            n.velocity = Vec2::ZERO;
        } else if !args.reduced_motion {
            let decay = math::exp(-5.5 * dt);
            movement = n.velocity * ((1. - decay) / 5.5);
            n.velocity *= decay;
            if n.velocity.length() < 0.001 {
                n.velocity = Vec2::ZERO;
            }
        }
        if args.reduced_motion {
            n.velocity = Vec2::ZERO;
        }
        n.down = down;
        let keys = input.stick("move");
        movement += Vec2::new(-keys.x, keys.y) * dt * 0.24;
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
        if n.velocity.length() > 0.001 {
            w.busy("orbital drift");
        }
        *w.resource_mut::<Navigation>() = n;
        draw(w, &n);
    }
}
