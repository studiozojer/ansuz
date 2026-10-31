use exact_game::*;
pub mod hologram;
pub mod records;
use hologram::Holograms;
use records::{Dataset, ROOT_IDS};

pub const RADIUS: f32 = 120.0;
pub const HIGH: f32 = 32.0;
pub const LOW: f32 = 14.0;
pub const PLACES: [(&str, f32, f32, &str); 5] = [
    ("The Returning", -0.06, 0.24, "#bcbcbc"),
    ("Ember Grove", 0.20, 0.36, "#bcbcbc"),
    ("Stillwater", -0.29, 0.43, "#bcbcbc"),
    ("The Unwritten", 0.40, 0.65, "#bcbcbc"),
    ("Open Questions", -0.48, 0.16, "#bcbcbc"),
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
/// The semantic reference is explicit; neither this entity name nor its cell is an ID.
#[derive(Default, Component)]
pub struct RootObject {
    pub id: String,
}
#[derive(Default, Data)]
struct Hud {
    title: String,
    detail: String,
    selected: bool,
    entered: bool,
    travelling: bool,
    altitude: f32,
    active: bool,
    hover_title: String,
    hover_id: String,
    solid_name: String,
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
    // Regional selection envelope; voxel-cell picking is not needed for these cluster targets.
    let b = pose.position.dot(ray);
    let c = pose.position.length_squared() - (RADIUS + 5.0) * (RADIUS + 5.0);
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
    const ASSETS: &'static [&'static str] = &["terrain.model", "pixel.tex"];
    type Args = Options;
    fn actions() -> Actions {
        Actions::new()
            .stick("move", Stick::wasd().or_arrows())
            .button("contact", &["MouseLeft"])
            .button("descend", &["Enter"])
            .button("ascend", &["Escape"])
            .button("next", &["KeyN"])
            .button("solid-next", &["KeyE"])
            .button("solid-prev", &["KeyQ"])
    }
    fn setup(w: &mut World, _: &Options) {
        let j = Journey {
            altitude: HIGH,
            selected: -1,
            ..Journey::default()
        };
        w.insert_resource(j);
        w.insert_resource(Dataset::demo());
        w.insert_resource(Environment {
            background: Some(Color::hex("#080808")),
            zenith: [0.15; 3],
            horizon: [0.15; 3],
            ground: [0.08; 3],
            ambient: 0.8,
            sun_disc: 0.,
            fog: None,
            bloom: None,
            ..Environment::default()
        });
        w.spawn_named(
            "terrain",
            (Transform::default(), Mesh::asset("terrain.model")),
        );
        w.spawn_named("camera", (camera_pose(&j), Camera::default()));
        // The volume is baked to exposed, merged faces. Readings remain independent entities.
        for (i, (_, x, y, color)) in PLACES.iter().enumerate() {
            w.spawn_named(
                format!("place-{i}"),
                (
                    surface(*x, *y, 6.0),
                    Mesh::cuboid(hologram::ROOT_SIZE),
                    Material::hex(color).emissive(0.3, 0.3, 0.3),
                    Marker { place: i as u32 },
                    RootObject {
                        id: ROOT_IDS[i].into(),
                    },
                ),
            );
            w.spawn_named(
                format!("halo-{i}"),
                (
                    surface(*x, *y, 5.25),
                    Mesh::cuboid(Vec3::new(2.4, 0.06, 2.4)),
                    Material::hex("#555555").emissive(0.05, 0.05, 0.05),
                ),
            );
            for k in 0..9 {
                let lon = x + ((k % 3) as f32 - 1.) * 0.028;
                let lat = y + ((k / 3) as f32 - 1.) * 0.028;
                w.spawn_named(
                    format!("reading-{i}-{k}"),
                    (
                        surface(lon, lat, 5.6),
                        Mesh::cuboid(Vec3::new(0.8, 0.5, 0.8)),
                        Material::hex("#888888").emissive(0.1, 0.1, 0.1),
                        Marker { place: i as u32 },
                    ),
                );
            }
            w.spawn_named(
                format!("motes-{i}"),
                (
                    surface(*x, *y, 7.),
                    Ambient,
                    ParticleLook {
                        texture: "pixel.tex".into(),
                        ..ParticleLook::default()
                    },
                    Emitter {
                        shape: exact_game::emitter::Shape::Sphere(4.),
                        rate: 3.,
                        lifetime: 5.,
                        speed: 0.15,
                        spread: 0.4,
                        gravity: Vec3::new(0., 0.04, 0.),
                        drag: 0.2,
                        size: [0.10, 0.06],
                        color: [[0.5, 0.5, 0.5, 0.8], [0.5, 0.5, 0.5, 0.]],
                        additive: false,
                        bound: [-5., -5., -5., 5., 7., 5.],
                        seed: i as u64 + 1,
                        ..Emitter::default()
                    },
                ),
            );
        }
        hologram::setup(w);
        publish(w, &j);
    }
    fn tick(w: &mut World, input: &Input, args: &Options) {
        for i in 0..PLACES.len() {
            w.require_mut::<Emitter>(format!("motes-{i}")).running = !args.reduced;
        }
        let dt = w.dt();
        let mut j = *w.resource::<Journey>();
        let mut holograms = *w.resource::<Holograms>();
        let pointer = input.pointer();
        let down = pointer.is_some_and(|p| p.down);
        hologram::update_hover(w, input, &j, &mut holograms);
        let consume_wheel = holograms.wheel(input.wheel().y, dt);
        if consume_wheel && input.wheel().length_squared() > 0. {
            j.vx = 0.;
            j.vy = 0.;
        }
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
            holograms.focused = j.selected;
            holograms.hovered = -1;
        }
        if input.pressed("solid-next") || input.pressed("solid-prev") {
            if holograms.active() < 0 && j.selected >= 0 {
                holograms.focused = j.selected;
            }
            holograms.cycle(if input.pressed("solid-next") { 1 } else { -1 });
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
            let wheel = if consume_wheel {
                Vec2::ZERO
            } else {
                input.wheel()
            };
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
        hologram::draw(w, &holograms, &j, args.reduced);
        *w.resource_mut::<Holograms>() = holograms;
        publish(w, &j);
    }
}
fn publish(w: &World, j: &Journey) {
    let data = w.resource::<Dataset>();
    let holograms = w.resource::<Holograms>();
    let active = if j.travelling || j.was_down {
        -1
    } else {
        holograms.active()
    };
    let title = if j.selected >= 0 {
        data.object(ROOT_IDS[j.selected as usize])
            .map_or("Untitled", |o| o.title.as_str())
    } else {
        "A landscape of readings"
    };
    let detail = if j.entered {
        "Hover a topic’s top. Scroll to change its solid; drag to explore."
    } else if j.selected >= 0 {
        "A sample topic · descend to explore its surface"
    } else {
        "Five sample topics inhabit this landscape. Choose one to descend."
    };
    w.publish_record(&Hud {
        title: title.into(),
        detail: detail.into(),
        selected: j.selected >= 0,
        entered: j.entered,
        travelling: j.travelling,
        altitude: j.altitude,
        active: active >= 0,
        hover_title: if active >= 0 {
            data.object(ROOT_IDS[active as usize])
                .map_or("Untitled", |o| o.title.as_str())
                .into()
        } else {
            String::new()
        },
        hover_id: if active >= 0 {
            ROOT_IDS[active as usize].into()
        } else {
            String::new()
        },
        solid_name: if active >= 0 {
            hologram::SOLID_NAMES[holograms.shapes[active as usize] as usize].into()
        } else {
            String::new()
        },
    });
}
