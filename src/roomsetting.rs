// src/roomsetting.rs

#[derive(Clone, Copy, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Room {
    pub width: f32,
    pub length: f32,
    pub height: f32,
    pub walls: Walls,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wall {
    Left,
    Right,
    Front,
    Back,
    Floor,
    Ceiling,
}

pub struct Walls {
    pub left_wall: MultiMaterial,
    pub right_wall: MultiMaterial,
    pub front_wall: MultiMaterial,
    pub back_wall: MultiMaterial,
    pub floor: MultiMaterial,
    pub ceiling: MultiMaterial,
}

pub trait RoomGeometry {
    fn volume(&self) -> f32;
    fn surface_area(&self) -> f32;
}

pub struct BoxRoom {
    pub structure: Room,
} 

impl RoomGeometry for BoxRoom {
    fn volume(&self) -> f32 {
        self.structure.width * self.structure.length * self.structure.height
    }

    fn surface_area(&self) -> f32 {
        2.0 * (
            self.structure.width * self.structure.length +
            self.structure.width * self.structure.height +
            self.structure.length * self.structure.height
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub absorption: f32,
    pub diffusion: f32,
}

pub const BRICK: Material = Material { absorption: 0.03, diffusion: 0.20 };
pub const WOOD: Material = Material { absorption: 0.15, diffusion: 0.25 };
pub const CONCRETE: Material = Material { absorption: 0.02, diffusion: 0.15 };
pub const DRYWALL: Material = Material { absorption: 0.10, diffusion: 0.10 };
#[derive(Debug, Clone)]
pub struct MultiMaterial {
    pub materials: Vec<(Material, f32)>,
}

impl MultiMaterial {
    pub fn absorption(&self) -> f32 {
        self.materials.iter().map(|(mat, weight)| mat.absorption * weight).sum()
    }

    pub fn diffusion(&self) -> f32 {
        self.materials.iter().map(|(mat, weight)| mat.diffusion * weight).sum()
    }
    pub fn new(materials: Vec<(Material, f32)>) -> Self {
        Self {
            materials, 
        }
    }
}

pub struct Source {
    pub position: Vec3, 
    pub yaw: f32,
    pub pitch: f32,
}

pub struct Listener {
    pub position: Vec3,
}

pub fn distance(a: &Vec3, b: &Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

pub struct DirectSound {
    pub delay: usize,
    pub gain: f32,
}

pub struct Reflection {
    pub delay: usize,
    pub gain: f32,
}

impl DirectSound {
    pub fn from(source: &Vec3, listener: &Vec3, sample_rate: f32) -> Self {
        let dist = distance(source, listener);
        let delay = (dist / 343.0 * sample_rate) as usize;
        let k = 0.8;
        let gain = (1.0 + dist * dist) / (dist * k).exp();
        Self { delay, gain }
    }
}

pub fn wall_absorption(room: &Room, wall: &Wall) -> f32 {
    match wall {
        Wall::Left => room.walls.left_wall.absorption(),
        Wall::Right => room.walls.right_wall.absorption(),
        Wall::Front => room.walls.front_wall.absorption(),
        Wall::Back => room.walls.back_wall.absorption(),
        Wall::Floor => room.walls.floor.absorption(),
        Wall::Ceiling => room.walls.ceiling.absorption(),
    }
}

pub fn wall_diffusion(room: &Room, wall: &Wall) -> f32 {
    match wall {
        Wall::Left => room.walls.left_wall.diffusion(),
        Wall::Right => room.walls.right_wall.diffusion(),
        Wall::Front => room.walls.front_wall.diffusion(),
        Wall::Back => room.walls.back_wall.diffusion(),
        Wall::Floor => room.walls.floor.diffusion(),
        Wall::Ceiling => room.walls.ceiling.diffusion(),
    }
}

pub struct WallReflection {
    pub primary: Reflection,
    pub secondary: Reflection,
}

pub fn wall_reflection(source: &Source, listener: &Vec3, sample_rate: f32, room: &Room, wall: Wall) -> WallReflection {
    // 1. Находим координаты мнимого источника (Image Source)
    let mirrored_source = match wall {
        Wall::Left => Vec3 { x: -source.position.x, y: source.position.y, z: source.position.z },
        Wall::Right => Vec3 { x: 2.0 * room.width - source.position.x, y: source.position.y, z: source.position.z },
        Wall::Front => Vec3 { x: source.position.x, y: -source.position.y, z: source.position.z },
        Wall::Back => Vec3 { x: source.position.x, y: 2.0 * room.length - source.position.y, z: source.position.z },
        Wall::Floor => Vec3 { x: source.position.x, y: source.position.y, z: -source.position.z },
        Wall::Ceiling => Vec3 { x: source.position.x, y: source.position.y, z: 2.0 * room.height - source.position.z },
    };

    // 2. Расчет геометрии и расстояния
    let dist = distance(&mirrored_source, listener);
    let delay = (dist / 343.0 * sample_rate) as usize;

    // Точка падения луча на стену (приблизительно средняя точка между источником и приемником проекции)
    // Для мнимого источника луч идет по прямой линии к слушателю. Находим вектор луча:
    let ray_dir = Vec3 {
        x: listener.x - source.position.x,
        y: listener.y - source.position.y,
        z: listener.z - source.position.z,
    };
    let ray_len = (ray_dir.x.powi(2) + ray_dir.y.powi(2) + ray_dir.z.powi(2)).sqrt().max(0.001);
    
    // Нормализуем вектор направления луча к стене
    let ray_norm = Vec3 { x: ray_dir.x / ray_len, y: ray_dir.y / ray_len, z: ray_dir.z / ray_len };

    // 3. Расчет вектора направления источника на основе Yaw и Pitch (в радианах)
    let yaw_rad = source.yaw.to_radians();
    let pitch_rad = source.pitch.to_radians();

    let look_dir = Vec3 {
        x: pitch_rad.cos() * yaw_rad.sin(),
        y: pitch_rad.cos() * yaw_rad.cos(),
        z: pitch_rad.sin(),
    };

    // Скалярное произведение вектора взгляда и вектора уходящего луча
    let dot_product = look_dir.x * ray_norm.x + look_dir.y * ray_norm.y + look_dir.z * ray_norm.z;
    
    // Модель направленности (Кардиоида): 0.5 * (1.0 + cos(theta))
    // Дает максимальную громкость спереди (1.0), падение до 0.5 по бокам и 0.0 строго сзади
    let directivity_factor = 0.5 * (1.0 + dot_product);

    // 4. Учет акустических параметров материала стены
    let absorption = wall_absorption(room, &wall);
    let diffusion = wall_diffusion(room, &wall);

    // Размер спектрального рассеивания (зависит от коэффициента диффузии)
    let spread = (diffusion * 200.0) as usize;

    // Затухание звука в воздухе и падение громкости по закону обратных квадратов
    let air = (-dist * 0.05).exp();
    let geometric_attenuation = 1.0 / (1.0 + dist * dist);

    // Финальный расчет Gain первичного (зеркального) отражения
    // Оно ослабляется поглощением (1 - absorption), рассеиванием (1 - diffusion) и направленностью источника
    let primary_gain = (1.0 - absorption) * (1.0 - diffusion) * air * geometric_attenuation * directivity_factor;

    // Финальный расчет Gain вторичного (рассеянного) отражения
    // Оно забирает энергию, которую забрала диффузия
    let secondary_gain = (1.0 - absorption) * diffusion * air * geometric_attenuation * directivity_factor;

    let main = Reflection { delay, gain: primary_gain };
    let diffuse = Reflection { delay: delay + spread, gain: secondary_gain };

    WallReflection { primary: main, secondary: diffuse }
}
