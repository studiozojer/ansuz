use exact_game::*;
pub mod records;
pub mod terrain_pick;
use records::{Dataset, Object, Relation};
use terrain_pick::Cell;

pub const RADIUS: f32 = 120.;
pub const START_HEIGHT: f32 = 28.;
pub const ORBIT_HEIGHT: f32 = 32.;
#[derive(Default, Args)]
pub struct Options {
    #[live]
    pub records: String,
    #[live]
    pub editing: bool,
}
#[derive(Clone, Copy, Resource)]
pub struct Journey {
    pub longitude: f32,
    pub latitude: f32,
    pub surface: bool,
    pub blend: f32,
    pub quiet: f32,
    pub x: f32,
    pub z: f32,
    pub altitude: f32,
    pub vx: f32,
    pub vz: f32,
    pub selected: Option<Cell>,
    pub hovered: Option<Cell>,
    pub was_down: bool,
    pub dragged: f32,
}
impl Default for Journey {
    fn default() -> Self {
        Self {
            longitude: 0.,
            latitude: 0.,
            surface: false,
            blend: 0.,
            quiet: 0.,
            x: 0.,
            z: 0.,
            altitude: START_HEIGHT,
            vx: 0.,
            vz: 0.,
            selected: None,
            hovered: None,
            was_down: false,
            dragged: 0.,
        }
    }
}
#[derive(Clone, Default, Data)]
pub struct FacePlacement {
    pub object_id: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
impl FacePlacement {
    pub fn cell(&self) -> Cell {
        Cell {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}
#[derive(Default, Data)]
pub struct Archive {
    pub version: u32,
    pub next_id: u64,
    pub objects: Vec<Object>,
    pub relations: Vec<Relation>,
    pub placements: Vec<FacePlacement>,
}
#[derive(Default, Resource)]
pub struct SurfaceRecords {
    pub placements: Vec<FacePlacement>,
    pub loaded: String,
    pub painted: Vec<Entity>,
}
#[derive(Default, Data)]
struct Hud {
    surface: bool,
    travelling: bool,
    selected: bool,
    occupied: bool,
    title: String,
    body: String,
    object_id: String,
    x: i32,
    y: i32,
    z: i32,
    count: u32,
}
pub fn normal(lon: f32, lat: f32) -> Vec3 {
    Vec3::new(
        math::sin(lon) * math::cos(lat),
        math::sin(lat),
        math::cos(lon) * math::cos(lat),
    )
}
pub fn camera_pose(j: &Journey) -> Transform {
    let n = normal(j.longitude, j.latitude);
    let north = Vec3::new(
        -math::sin(j.longitude) * math::sin(j.latitude),
        math::cos(j.latitude),
        -math::cos(j.longitude) * math::sin(j.latitude),
    );
    let eye = n * (RADIUS + ORBIT_HEIGHT);
    let orbit = Transform::at(eye.x, eye.y, eye.z)
        .looking_at(eye + north * math::cos(0.82) - n * math::sin(0.82), n);
    let surface_y = math::sqrt((RADIUS * RADIUS - j.x * j.x - j.z * j.z).max(1.));
    let top_eye = Vec3::new(j.x, surface_y + j.altitude, j.z);
    let top = Transform::at(top_eye.x, top_eye.y, top_eye.z)
        .looking_at(Vec3::new(j.x, surface_y, j.z), -Vec3::Z);
    if j.blend <= 0. {
        return orbit;
    }
    if j.blend >= 1. {
        return top;
    }
    let t = j.blend * j.blend * (3. - 2. * j.blend);
    // Follow an arc outside the terrain instead of cutting through the sphere.
    let arc = Quat::IDENTITY.slerp(
        Quat::from_rotation_arc(eye.normalize(), top_eye.normalize()),
        t,
    );
    Transform {
        position: arc * eye.normalize() * (eye.length() + (top_eye.length() - eye.length()) * t),
        rotation: orbit.rotation.slerp(top.rotation, t),
        ..Transform::default()
    }
}
pub fn descend(j: &mut Journey) {
    if j.surface {
        return;
    }
    if let Some(cell) = j.selected {
        let p = cell.center();
        j.x = p.x;
        j.z = p.z;
    } else {
        j.x = 0.;
        j.z = 0.;
    }
    j.altitude = START_HEIGHT;
    j.surface = true;
    j.vx = 0.;
    j.vz = 0.;
    j.hovered = None;
}
pub fn ascend(j: &mut Journey) {
    j.surface = false;
    j.vx = 0.;
    j.vz = 0.;
    j.hovered = None;
}
pub fn pick(w: &World, point: Vec2, size: Vec2, j: &Journey) -> Option<Cell> {
    if size.min_element() <= 0.
        || point.x < 0.
        || point.y < 0.
        || point.x > size.x
        || point.y > size.y
    {
        return None;
    }
    let pose = camera_pose(j);
    let ndc = Vec3::new(2. * point.x / size.x - 1., 1. - 2. * point.y / size.y, 1.);
    let ray = pose.rotation
        * Camera::default()
            .matrix(size)
            .inverse()
            .project_point3(ndc)
            .normalize();
    terrain_pick::pick(w, pose.position, ray)
}
fn face_pose(cell: Cell, lift: f32) -> Transform {
    let p = cell.center() + Vec3::Y * lift;
    Transform::at(p.x, p.y, p.z)
}
fn sync_records(w: &mut World, json: &str) {
    if w.resource::<SurfaceRecords>().loaded == json {
        return;
    }
    let Ok(archive) = exact_game::json::from_str::<Archive>(json) else {
        return;
    };
    let painted = std::mem::take(&mut w.resource_mut::<SurfaceRecords>().painted);
    for entity in painted {
        w.despawn(entity);
    }
    *w.resource_mut::<Dataset>() = Dataset {
        objects: archive.objects,
        relations: archive.relations,
    };
    let mut painted = Vec::new();
    for placement in &archive.placements {
        painted.push(w.spawn((
            face_pose(placement.cell(), 0.007),
            Mesh::cuboid(Vec3::new(2.38, 0.004, 2.38)),
            Material::hex("#65777b").emissive(0.15, 0.2, 0.21),
        )));
    }
    *w.resource_mut::<SurfaceRecords>() = SurfaceRecords {
        placements: archive.placements,
        loaded: json.into(),
        painted,
    };
}
fn draw(w: &mut World, j: &Journey) {
    for (name, cell, lift) in [
        ("hover-face", j.hovered, 0.015),
        ("selected-face", j.selected, 0.023),
    ] {
        w.require_mut::<Visible>(name).0 = cell.is_some();
        if let Some(cell) = cell {
            *w.require_mut::<Transform>(name) = face_pose(cell, lift);
        }
    }
    *w.require_mut::<Transform>("camera") = camera_pose(j);
    let records = w.resource::<SurfaceRecords>();
    let dataset = w.resource::<Dataset>();
    let object = j
        .selected
        .and_then(|cell| records.placements.iter().find(|p| p.cell() == cell))
        .and_then(|p| dataset.object(&p.object_id));
    let cell = j.selected.unwrap_or_default();
    w.publish_record(&Hud {
        surface: j.surface,
        travelling: j.blend > 0. && j.blend < 1.,
        selected: j.selected.is_some(),
        occupied: object.is_some(),
        title: object.map_or("", |o| o.title.as_str()).into(),
        body: object.map_or("", |o| o.body.as_str()).into(),
        object_id: object.map_or("", |o| o.id.as_str()).into(),
        x: cell.x,
        y: cell.y,
        z: cell.z,
        count: w.resource::<Dataset>().objects.len() as u32,
    });
}
pub struct Ansuz;
impl Game for Ansuz {
    const ID: &'static str = "ansuz";
    const HZ: u32 = 120;
    const ASSETS: &'static [&'static str] = &["terrain.model"];
    type Args = Options;
    fn actions() -> Actions {
        Actions::new()
            .stick("move", Stick::wasd().or_arrows())
            .button("contact", &["MouseLeft"])
            .button("recenter", &["Home"])
            .button("descend", &["KeyF"])
            .button("ascend", &["KeyR"])
            .button("clear", &["Escape"])
            .button("select-center", &["Enter"])
            .button("closer", &["Equal"])
            .button("farther", &["Minus"])
    }
    fn setup(w: &mut World, args: &Options) {
        w.insert_resource(Journey::default());
        w.insert_resource(Dataset::default());
        w.insert_resource(SurfaceRecords::default());
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
        w.spawn_named(
            "camera",
            (camera_pose(&Journey::default()), Camera::default()),
        );
        for (name, color) in [("hover-face", "#567076"), ("selected-face", "#9ab8be")] {
            w.spawn_named(
                name,
                (
                    Transform::default(),
                    Mesh::cuboid(Vec3::new(2.38, 0.004, 2.38)),
                    Material::hex(color).emissive(0.15, 0.20, 0.21),
                    Visible(false),
                ),
            );
        }
        terrain_pick::setup(w);
        sync_records(w, &args.records);
        draw(w, &Journey::default());
    }
    fn tick(w: &mut World, input: &Input, args: &Options) {
        sync_records(w, &args.records);
        let mut j = *w.resource::<Journey>();
        let dt = w.dt();
        if args.editing {
            j.vx = 0.;
            j.vz = 0.;
            j.hovered = None;
            j.was_down = false;
            *w.resource_mut::<Journey>() = j;
            draw(w, &j);
            return;
        }
        let pointer = input.pointer();
        let down = pointer.is_some_and(|p| p.down);
        if input.pressed("recenter") {
            j = Journey::default();
        }
        if input.pressed("clear") {
            j.selected = None;
        }
        if input.pressed("descend") {
            descend(&mut j);
        }
        if input.pressed("ascend") {
            ascend(&mut j);
        }
        let target = if j.surface { 1. } else { 0. };
        if j.blend != target {
            j.blend = if j.surface {
                (j.blend + dt / 0.85).min(1.)
            } else {
                (j.blend - dt / 0.85).max(0.)
            };
            j.hovered = None;
            j.was_down = false;
            w.busy("camera transitioning");
            *w.resource_mut::<Journey>() = j;
            draw(w, &j);
            return;
        }
        if input.pressed("contact") {
            j.vx = 0.;
            j.vz = 0.;
            j.dragged = 0.;
            j.quiet = 0.;
        }
        if down {
            j.quiet = (j.quiet + dt).min(0.2);
        }
        if down && j.was_down {
            if let Some(p) = pointer {
                j.dragged += p.delta.length();
                if j.surface {
                    let scale = j.altitude * 0.85 / input.viewport().y.max(320.);
                    j.x -= p.delta.x * scale;
                    j.z -= p.delta.y * scale;
                } else if p.delta.length_squared() > 0. {
                    let scale = ORBIT_HEIGHT / RADIUS * 1.25 / input.viewport().y.max(320.);
                    let dx = -p.delta.x * scale / math::cos(j.latitude).max(0.3);
                    let dy = p.delta.y * scale;
                    j.longitude += dx;
                    j.latitude += dy;
                    j.vx = (dx / j.quiet.max(dt)).clamp(-1.2, 1.2);
                    j.vz = (dy / j.quiet.max(dt)).clamp(-1.2, 1.2);
                    j.quiet = 0.;
                }
            }
        }
        if !down {
            let stick = input.stick("move");
            let wheel = input.wheel();
            if j.surface {
                j.x += stick.x * j.altitude * dt * 0.7;
                j.z -= stick.y * j.altitude * dt * 0.7;
                j.altitude = (j.altitude + wheel.y * 0.025).clamp(12., 70.);
                if input.pressed("closer") {
                    j.altitude = (j.altitude - 5.).max(12.);
                }
                if input.pressed("farther") {
                    j.altitude = (j.altitude + 5.).min(70.);
                }
                let radius = math::sqrt(j.x * j.x + j.z * j.z);
                if radius > 124. {
                    j.x *= 124. / radius;
                    j.z *= 124. / radius;
                }
            } else {
                if stick.length_squared() > 0. {
                    j.vx = stick.x * ORBIT_HEIGHT / RADIUS * 0.55;
                    j.vz = stick.y * ORBIT_HEIGHT / RADIUS * 0.55;
                }
                j.longitude += j.vx * dt + wheel.x * ORBIT_HEIGHT / RADIUS * 0.001;
                j.latitude += j.vz * dt - wheel.y * ORBIT_HEIGHT / RADIUS * 0.001;
                let friction = math::exp(-4.8 * dt);
                j.vx *= friction;
                j.vz *= friction;
                if j.vx.abs() < 0.0001 {
                    j.vx = 0.;
                }
                if j.vz.abs() < 0.0001 {
                    j.vz = 0.;
                }
            }
        }
        if input.released("contact") && j.quiet > 0.09 {
            j.vx = 0.;
            j.vz = 0.;
        }
        if j.was_down && !down && !input.released("contact") {
            j.vx = 0.;
            j.vz = 0.;
        }
        j.longitude = (j.longitude + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        j.latitude = j.latitude.clamp(-1.15, 1.15);
        if j.vx != 0. || j.vz != 0. {
            w.busy("camera coasting");
        }
        j.hovered = if down {
            None
        } else {
            pointer.and_then(|p| pick(w, p.position, input.viewport(), &j))
        };
        if input.released("contact") {
            if let Some(p) = pointer {
                let travel = p.press_origin.map_or(j.dragged, |origin| {
                    (p.position - origin).length().max(j.dragged)
                });
                if travel < 7. {
                    j.selected = pick(w, p.position, input.viewport(), &j);
                    j.vx = 0.;
                    j.vz = 0.;
                }
            }
        }
        if input.pressed("select-center") {
            j.selected = pick(w, input.viewport() * 0.5, input.viewport(), &j);
        }
        j.was_down = down;
        *w.resource_mut::<Journey>() = j;
        draw(w, &j);
    }
}
