use exact_game::*;
pub mod records;
pub mod terrain_pick;
use records::{Dataset, Object, Relation};
use terrain_pick::Cell;

pub const RADIUS: f32 = 120.;
pub const START_HEIGHT: f32 = 9.;
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
    pub focus: Cell,
    pub from: Transform,
    pub travel: f32,
    pub quiet: f32,
    /// Pan offsets in the focused face's right/up basis.
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
            latitude: 1.05,
            surface: false,
            focus: Cell::default(),
            from: Transform::default(),
            travel: 1.,
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
    pub face: u32,
}
impl FacePlacement {
    pub fn cell(&self) -> Cell {
        Cell {
            x: self.x,
            y: self.y,
            z: self.z,
            face: self.face,
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
    face: u32,
    count: u32,
}
pub fn normal(lon: f32, lat: f32) -> Vec3 {
    Vec3::new(
        math::sin(lon) * math::cos(lat),
        math::sin(lat),
        math::cos(lon) * math::cos(lat),
    )
}
fn destination(j: &Journey) -> Transform {
    if j.surface {
        let center = j.focus.center() + j.focus.right() * j.x + j.focus.up() * j.z;
        let eye = center + j.focus.normal() * j.altitude;
        Transform::at(eye.x, eye.y, eye.z).looking_at(center, j.focus.up())
    } else {
        let n = normal(j.longitude, j.latitude);
        let north = Vec3::new(
            -math::sin(j.longitude) * math::sin(j.latitude),
            math::cos(j.latitude),
            -math::cos(j.longitude) * math::sin(j.latitude),
        );
        let eye = n * (RADIUS + ORBIT_HEIGHT);
        Transform::at(eye.x, eye.y, eye.z)
            .looking_at(eye + north * math::cos(0.82) - n * math::sin(0.82), n)
    }
}
pub fn camera_pose(j: &Journey) -> Transform {
    let to = destination(j);
    if j.travel >= 1. {
        return to;
    }
    if j.travel <= 0. {
        return j.from;
    }
    let t = j.travel * j.travel * (3. - 2. * j.travel);
    let a = j.from.position;
    let b = to.position;
    let position = if a.distance(b) < 20. {
        a.lerp(b, t)
    } else {
        // Distant moves lift around the landscape rather than cutting a chord
        // through its centre. Nearby moves stay local to the selected faces.
        let arc = Quat::IDENTITY.slerp(Quat::from_rotation_arc(a.normalize(), b.normalize()), t);
        let lift = ((RADIUS + 40.) - a.length().min(b.length())).max(0.)
            * math::sin(std::f32::consts::PI * t);
        arc * a.normalize() * (a.length() + (b.length() - a.length()) * t + lift)
    };
    Transform {
        position,
        rotation: j.from.rotation.slerp(to.rotation, t),
        ..Transform::default()
    }
}
fn stop(j: &mut Journey) {
    j.vx = 0.;
    j.vz = 0.;
    j.hovered = None;
    j.was_down = false;
}
pub fn focus_surface(w: &World, j: &mut Journey, cell: Cell) {
    let from = camera_pose(j);
    j.focus = cell;
    j.selected = Some(cell);
    j.surface = true;
    j.x = 0.;
    j.z = 0.;
    j.altitude = clearance(w, cell.center(), cell.normal(), START_HEIGHT);
    j.from = from;
    j.travel = 0.;
    stop(j);
}
pub fn ascend(j: &mut Journey) {
    j.from = camera_pose(j);
    j.surface = false;
    j.travel = 0.;
    j.selected = None;
    stop(j);
}
fn clearance(w: &World, center: Vec3, normal: Vec3, wanted: f32) -> f32 {
    match terrain_pick::hit(w, center + normal * 0.03, normal) {
        Some((distance, Some(_))) => wanted.min((distance * 0.45).max(0.15)),
        // A pan can put the reference plane below a higher neighbouring face.
        // Do not pull the eye into that solid; the movement sweep below stops it.
        _ => wanted,
    }
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
    Transform {
        position: cell.center() + cell.normal() * lift,
        rotation: cell.rotation() * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        ..Transform::default()
    }
}
fn edge_name(prefix: &str, index: usize) -> String {
    if index == 0 {
        prefix.into()
    } else {
        format!("{prefix}-{index}")
    }
}
fn frame_edges(width: f32) -> [(Vec3, Vec3); 4] {
    let half = (2.38 - width) * 0.5;
    [
        (Vec3::new(0., 0., -half), Vec3::new(2.38, 0.008, width)),
        (Vec3::new(0., 0., half), Vec3::new(2.38, 0.008, width)),
        (Vec3::new(-half, 0., 0.), Vec3::new(width, 0.008, 2.38)),
        (Vec3::new(half, 0., 0.), Vec3::new(width, 0.008, 2.38)),
    ]
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
            Material::hex("#303a48").emissive(0.018, 0.025, 0.035),
        )));
        let mut mark = face_pose(placement.cell(), 0.018);
        mark.position += mark.rotation * Vec3::new(0.72, 0., 0.72);
        painted.push(w.spawn((
            mark,
            Mesh::cuboid(Vec3::new(0.32, 0.006, 0.32)),
            Material::hex("#a2aab4").emissive(0.08, 0.09, 0.11),
        )));
    }
    *w.resource_mut::<SurfaceRecords>() = SurfaceRecords {
        placements: archive.placements,
        loaded: json.into(),
        painted,
    };
}
fn draw(w: &mut World, j: &Journey) {
    w.require_mut::<Visible>("surface-writing").0 = j.selected.is_some();
    if let Some(cell) = j.selected {
        *w.require_mut::<Transform>("surface-writing") = Transform {
            position: cell.center() + cell.normal() * 0.02,
            rotation: cell.rotation(),
            ..Transform::default()
        };
    }
    let hovered = j.hovered.filter(|cell| Some(*cell) != j.selected);
    for (prefix, cell, width, lift) in [
        ("hover-face", hovered, 0.045, 0.025),
        ("selected-face", j.selected, 0.11, 0.035),
    ] {
        for (index, (offset, _)) in frame_edges(width).into_iter().enumerate() {
            let name = edge_name(prefix, index);
            w.require_mut::<Visible>(&name).0 = cell.is_some();
            if let Some(cell) = cell {
                let mut pose = face_pose(cell, lift);
                pose.position += pose.rotation * offset;
                *w.require_mut::<Transform>(&name) = pose;
            }
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
        travelling: j.travel < 1.,
        selected: j.selected.is_some(),
        occupied: object.is_some(),
        title: object.map_or("", |o| o.title.as_str()).into(),
        body: object.map_or("", |o| o.body.as_str()).into(),
        object_id: object.map_or("", |o| o.id.as_str()).into(),
        x: cell.x,
        y: cell.y,
        z: cell.z,
        face: cell.face,
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
            .button("ascend", &["KeyR"])
            .button("clear", &["Escape"])
            .button("select-center", &["Enter", "KeyF"])
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
        for (prefix, color, glow, width) in [
            ("hover-face", "#77dce8", [0.10, 0.34, 0.38], 0.045),
            ("selected-face", "#edb76b", [0.50, 0.24, 0.065], 0.11),
        ] {
            for (index, (_, size)) in frame_edges(width).into_iter().enumerate() {
                w.spawn_named(
                    &edge_name(prefix, index),
                    (
                        Transform::default(),
                        Mesh::cuboid(size),
                        Material::hex(color).emissive(glow[0], glow[1], glow[2]),
                        Visible(false),
                    ),
                );
            }
        }
        terrain_pick::setup(w);
        w.spawn_named(
            "surface-writing",
            (
                Transform::default(),
                Placed {
                    anchor: [0.5, 0.5],
                    ..Placed::child("surface-content")
                        .width(2.0)
                        .facing(Facing::Fixed)
                },
                Visible(false),
            ),
        );
        sync_records(w, &args.records);
        draw(w, &Journey::default());
    }
    fn tick(w: &mut World, input: &Input, args: &Options) {
        sync_records(w, &args.records);
        let mut j = *w.resource::<Journey>();
        let previous = j;
        let dt = w.dt();
        if args.editing {
            stop(&mut j);
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
        if input.pressed("ascend") && j.surface {
            ascend(&mut j);
        }
        if j.travel < 1. {
            j.travel = (j.travel + dt / 0.9).min(1.);
            stop(&mut j);
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
                    j.z += p.delta.y * scale;
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
                j.z += stick.y * j.altitude * dt * 0.7;
                j.altitude = (j.altitude + wheel.y * 0.015).clamp(0.3, 28.);
                if input.pressed("closer") {
                    j.altitude = (j.altitude - 2.).max(0.3);
                }
                if input.pressed("farther") {
                    j.altitude = (j.altitude + 2.).min(28.);
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
        if j.surface {
            // Stay near the focused patch while panning; a different face can
            // be selected to establish another local plane.
            j.x = j.x.clamp(-10., 10.);
            j.z = j.z.clamp(-10., 10.);
            let center = j.focus.center() + j.focus.right() * j.x + j.focus.up() * j.z;
            j.altitude = clearance(w, center, j.focus.normal(), j.altitude);
            let before = camera_pose(&previous).position;
            let movement = camera_pose(&j).position - before;
            if movement.length() > 0.00001
                && terrain_pick::hit(w, before, movement.normalize())
                    .is_some_and(|(distance, _)| distance < movement.length() + 0.12)
            {
                j.x = previous.x;
                j.z = previous.z;
                j.altitude = previous.altitude;
            }
        }
        if input.released("contact") && j.quiet > 0.09
            || j.was_down && !down && !input.released("contact")
        {
            j.vx = 0.;
            j.vz = 0.;
        }
        j.longitude = (j.longitude + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        j.latitude = j.latitude.clamp(-1.55, 1.55);
        if j.vx != 0. || j.vz != 0. {
            w.busy("camera coasting");
        }
        j.hovered = if down {
            None
        } else {
            pointer.and_then(|p| pick(w, p.position, input.viewport(), &j))
        };
        let click = if input.released("contact") && !input.pressed("clear") {
            pointer
                .filter(|p| {
                    p.press_origin
                        .is_some_and(|origin| (p.position - origin).length().max(j.dragged) < 7.)
                })
                .map(|p| p.position)
        } else {
            None
        };
        let point = if input.pressed("select-center") {
            Some(input.viewport() * 0.5)
        } else {
            click
        };
        if let Some(point) = point {
            if let Some(cell) = pick(w, point, input.viewport(), &j) {
                focus_surface(w, &mut j, cell);
            }
        }
        j.was_down = down;
        *w.resource_mut::<Journey>() = j;
        draw(w, &j);
    }
}
