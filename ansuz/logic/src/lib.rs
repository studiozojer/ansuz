use exact_game::*;

pub const RADIUS: f32 = 120.0;
pub const HIGH: f32 = 25.0;
pub const LOW: f32 = 6.5;
pub const PLACES: [(&str, f32, f32, &str); 5] = [
    ("The Returning", -0.06, 0.24, "#e6c38b"),
    ("Ember Grove", 0.20, 0.36, "#d99573"),
    ("Stillwater", -0.29, 0.43, "#9cc9c6"),
    ("The Unwritten", 0.40, 0.65, "#c2b6d9"),
    ("Open Questions", -0.48, 0.16, "#b5c795"),
];

#[derive(Default, Args)]
pub struct Options {
    #[restart]
    pub restart: bool,
    #[live]
    pub reduced: bool,
}

#[derive(Clone, Copy, Default, Resource)]
pub struct Journey {
    pub longitude: f32,
    pub latitude: f32,
    pub altitude: f32,
    pub vx: f32,
    pub vy: f32,
    pub selected: i32,
    pub entered: bool,
    pub travelling: bool,
    pub target_x: f32,
    pub target_y: f32,
    pub target_h: f32,
    pub return_x: f32,
    pub return_y: f32,
    pub return_h: f32,
    pub was_down: bool,
    pub dragged: f32,
    pub quiet: f32,
}
#[derive(Default, Component)]
pub struct Marker {
    pub place: u32,
}
#[derive(Default, Data)]
struct Hud {
    title: String,
    detail: String,
    selected: bool,
    entered: bool,
    travelling: bool,
    altitude: f32,
}

pub fn normal(lon: f32, lat: f32) -> Vec3 {
    Vec3::new(
        math::sin(lon) * math::cos(lat),
        math::sin(lat),
        math::cos(lon) * math::cos(lat),
    )
}
fn surface(lon: f32, lat: f32, height: f32) -> Transform {
    let n = normal(lon, lat);
    Transform {
        position: n * (RADIUS + height),
        rotation: Quat::from_rotation_arc(Vec3::Y, n),
        ..Transform::default()
    }
}
pub fn camera_pose(j: &Journey) -> Transform {
    let n = normal(j.longitude, j.latitude);
    let north = Vec3::new(
        -math::sin(j.longitude) * math::sin(j.latitude),
        math::cos(j.latitude),
        -math::cos(j.longitude) * math::sin(j.latitude),
    );
    let t = ((HIGH - j.altitude) / (HIGH - LOW)).clamp(0., 1.);
    let pitch = 0.82 + t * 0.51;
    let eye = n * (RADIUS + j.altitude);
    Transform::at(eye.x, eye.y, eye.z)
        .looking_at(eye + north * math::cos(pitch) - n * math::sin(pitch), n)
}
/// A damped coast expressed in seconds, independent of render frame rate.
pub fn decay(dt: f32) -> f32 {
    math::exp(-4.8 * dt)
}
fn wrap(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
pub fn begin_descent(j: &mut Journey) {
    if j.selected < 0 || j.entered || j.travelling {
        return;
    }
    let (_, x, y, _) = PLACES[j.selected as usize];
    j.return_x = j.longitude;
    j.return_y = j.latitude;
    j.return_h = j.altitude;
    j.target_x = x;
    j.target_y = y - 0.014;
    j.target_h = LOW;
    j.vx = 0.;
    j.vy = 0.;
    j.entered = true;
    j.travelling = true;
}
pub fn begin_ascent(j: &mut Journey) {
    if !j.entered {
        return;
    }
    j.target_x = j.return_x;
    j.target_y = j.return_y;
    j.target_h = j.return_h;
    j.vx = 0.;
    j.vy = 0.;
    j.entered = false;
    j.travelling = true;
}
fn pick_place(point: Vec2, size: Vec2, j: &Journey) -> i32 {
    if size.min_element() <= 0. {
        return -1;
    }
    let pose = camera_pose(j);
    let inverse = Camera::default().matrix(size).inverse();
    let ndc = Vec3::new(point.x / size.x * 2. - 1., 1. - point.y / size.y * 2., 1.);
    let ray = pose.rotation * inverse.project_point3(ndc).normalize();
    // Ray against the actual spherical surface. The near intersection excludes the far side.
    let b = pose.position.dot(ray);
    let c = pose.position.length_squared() - (RADIUS + 0.8) * (RADIUS + 0.8);
    let discriminant = b * b - c;
    if discriminant < 0. {
        return -1;
    }
    let distance = -b - math::sqrt(discriminant);
    if distance <= 0. {
        return -1;
    }
    let hit = (pose.position + ray * distance).normalize();
    let mut result = -1;
    let mut nearest = 0.12;
    for (i, (_, x, y, _)) in PLACES.iter().enumerate() {
        let d = (hit - normal(*x, *y)).length();
        if d < nearest {
            nearest = d;
            result = i as i32;
        }
    }
    result
}

pub struct Ansuz;
impl Game for Ansuz {
    const ID: &'static str = "ansuz";
    const HZ: u32 = 120;
    type Args = Options;
    fn actions() -> Actions {
        Actions::new()
            .stick("move", Stick::wasd().or_arrows())
            .button("contact", &["MouseLeft"])
            .button("descend", &["Enter"])
            .button("ascend", &["Escape"])
            .button("next", &["KeyN"])
    }
    fn setup(w: &mut World, _: &Options) {
        let j = Journey {
            altitude: HIGH,
            selected: -1,
            ..Journey::default()
        };
        w.insert_resource(j);
        w.insert_resource(Environment {
            zenith: [0.012, 0.028, 0.045],
            horizon: [0.21, 0.34, 0.34],
            ground: [0.035, 0.07, 0.065],
            ambient: 0.85,
            sun_disc: 0.,
            fog: Some(Fog {
                color: Some(Color::hex("#708f90")),
                ..Fog::new(0.0004, 0.)
            }),
            ..Environment::default()
        });
        w.spawn_named(
            "ocean",
            (
                Transform::default(),
                Mesh::asset("ocean.model"),
                Material::hex("#23494b"),
            ),
        );
        w.spawn_named("camera", (camera_pose(&j), Camera::default()));
        w.spawn_named(
            "sun",
            (
                Transform::at(-110., 180., 190.).looking_at(Vec3::ZERO, Vec3::Y),
                DirectionalLight::default(),
            ),
        );
        // Fixed archipelagos. Sample clusters have recognizable shapes and never drift.
        for (i, (_, x, y, color)) in PLACES.iter().enumerate() {
            for k in 0..5 {
                let a = k as f32 * 1.8 + i as f32;
                let lon = x + math::sin(a) * 0.035;
                let lat = y + math::cos(a) * 0.025;
                w.spawn((
                    surface(lon, lat, -1.9).with_scale([9. + k as f32 * 0.7, 3.5, 7.5]),
                    Mesh::sphere(1.),
                    Material::hex(if i % 2 == 0 { "#6f8670" } else { "#8a8c70" }),
                ));
            }
            w.spawn_named(
                format!("place-{i}"),
                (
                    surface(*x, *y, 2.1),
                    Mesh::cylinder(1.05, 0.3),
                    Material::hex(color).emissive(0.12, 0.10, 0.06),
                    Marker { place: i as u32 },
                ),
            );
            w.spawn_named(
                format!("halo-{i}"),
                (
                    surface(*x, *y, 2.3),
                    Mesh::ring(1.8, 1.95),
                    Material::hex(color),
                ),
            );
            for k in 0..9 {
                let a = k as f32 * 2.4;
                let r = 0.018 + (k / 3) as f32 * 0.012;
                let lon = x + math::cos(a) * r;
                let lat = y + math::sin(a) * r;
                w.spawn_named(
                    format!("reading-{i}-{k}"),
                    (
                        surface(lon, lat, 1.8),
                        Mesh::cuboid(Vec3::new(0.95, 0.2 + (k % 3) as f32 * 0.12, 1.2)),
                        Material::hex(color),
                        Marker { place: i as u32 },
                    ),
                );
                // Small cairns suggest an inhabited landscape without prescribing villages.
                if k % 3 == 0 {
                    w.spawn((
                        surface(lon + 0.008, lat, 1.8),
                        Mesh::cone(0.45, 1.6),
                        Material::hex("#3f6356"),
                    ));
                }
            }
        }
        publish(w, &j);
    }
    fn tick(w: &mut World, input: &Input, args: &Options) {
        let dt = w.dt();
        let mut j = *w.resource::<Journey>();
        let pointer = input.pointer();
        let down = pointer.is_some_and(|p| p.down);
        if input.pressed("contact") {
            j.vx = 0.;
            j.vy = 0.;
            j.dragged = 0.;
            j.quiet = 0.;
            j.travelling = false;
        }
        if down {
            j.quiet = (j.quiet + dt).min(0.2);
        }
        if down {
            if let Some(p) = pointer {
                let d = if j.was_down { p.delta } else { Vec2::ZERO };
                if d.length_squared() > 0. {
                    j.dragged += d.length();
                    let scale = j.altitude / RADIUS * 1.25 / input.viewport().y.max(320.);
                    let dx = -d.x * scale / math::cos(j.latitude).max(0.3);
                    let dy = d.y * scale;
                    let elapsed = j.quiet.max(dt);
                    j.vx = (dx / elapsed).clamp(-1.2, 1.2);
                    j.vy = (dy / elapsed).clamp(-1.2, 1.2);
                    j.longitude += dx;
                    j.latitude += dy;
                    j.quiet = 0.;
                }
            }
        }
        if input.released("contact") {
            if let Some(p) = pointer {
                let travel = p.press_origin.map_or(j.dragged, |origin| {
                    (p.position - origin).length().max(j.dragged)
                });
                if travel < 9. {
                    j.selected = pick_place(p.position, input.viewport(), &j);
                    j.vx = 0.;
                    j.vy = 0.;
                }
            }
            if j.quiet > 0.09 {
                j.vx = 0.;
                j.vy = 0.;
            }
        }
        // Blur/cancel must not leave the world coasting from a lost contact.
        if j.was_down && !down && !input.released("contact") {
            j.vx = 0.;
            j.vy = 0.;
        }
        if input.pressed("next") {
            j.selected = (j.selected + 1) % PLACES.len() as i32;
        }
        if input.pressed("descend") {
            begin_descent(&mut j);
        }
        if input.pressed("ascend") {
            begin_ascent(&mut j);
        }
        if j.travelling {
            let a = if args.reduced {
                1.
            } else {
                1. - math::exp(-5. * dt)
            };
            j.longitude += wrap(j.target_x - j.longitude) * a;
            j.latitude += (j.target_y - j.latitude) * a;
            j.altitude += (j.target_h - j.altitude) * a;
            if wrap(j.target_x - j.longitude).abs() < 0.0001
                && (j.target_y - j.latitude).abs() < 0.0001
                && (j.target_h - j.altitude).abs() < 0.01
            {
                j.longitude = j.target_x;
                j.latitude = j.target_y;
                j.altitude = j.target_h;
                j.travelling = false;
            }
        } else if !down {
            let stick = input.stick("move");
            let wheel = input.wheel();
            if stick.length_squared() > 0. {
                j.vx = stick.x * j.altitude / RADIUS * 0.55;
                j.vy = stick.y * j.altitude / RADIUS * 0.55;
            }
            j.longitude += j.vx * dt + wheel.x * j.altitude / RADIUS * 0.001;
            j.latitude += j.vy * dt - wheel.y * j.altitude / RADIUS * 0.001;
            let friction = if args.reduced { 0. } else { decay(dt) };
            j.vx *= friction;
            j.vy *= friction;
            if j.vx.abs() < 0.0001 {
                j.vx = 0.;
            }
            if j.vy.abs() < 0.0001 {
                j.vy = 0.;
            }
        }
        j.longitude = wrap(j.longitude);
        j.latitude = j.latitude.clamp(-1.15, 1.15);
        j.was_down = down;
        if j.travelling || j.vx != 0. || j.vy != 0. {
            w.busy("camera moving");
        }
        *w.require_mut::<Transform>("camera") = camera_pose(&j);
        for i in 0..PLACES.len() {
            w.require_mut::<Transform>(format!("halo-{i}")).scale =
                Vec3::splat(if j.selected == i as i32 { 1.65 } else { 1. });
        }
        *w.resource_mut::<Journey>() = j;
        publish(w, &j);
    }
}
fn publish(w: &World, j: &Journey) {
    let title = if j.selected >= 0 {
        PLACES[j.selected as usize].0
    } else {
        "A landscape of readings"
    };
    let detail = if j.entered {
        "I Ching · Return / Rune · Ansuz / A note left open"
    } else if j.selected >= 0 {
        "9 sample readings · a place to explore"
    } else {
        "Glide across the surface. Touch a clearing to choose a place."
    };
    w.publish_record(&Hud {
        title: title.into(),
        detail: detail.into(),
        selected: j.selected >= 0,
        entered: j.entered,
        travelling: j.travelling,
        altitude: j.altitude,
    });
}
