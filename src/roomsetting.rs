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

pub fn wall_reflection(source: &Vec3, listener: &Vec3, sample_rate: f32, room: &Room, wall: Wall) -> WallReflection {
    let mirrored_source = match wall {
        Wall::Left => Vec3 { x: -source.x, y: source.y, z: source.z },
        Wall::Right => Vec3 { x: 2.0 * room.width - source.x, y: source.y, z: source.z },
        Wall::Front => Vec3 { x: source.x, y: -source.y, z: source.z },
        Wall::Back => Vec3 { x: source.x, y: 2.0 * room.length - source.y, z: source.z },
        Wall::Floor => Vec3 { x: source.x, y: source.y, z: -source.z },
        Wall::Ceiling => Vec3 { x: source.x, y: source.y, z: 2.0 * room.height - source.z },
    };

    let dist = distance(&mirrored_source, listener);
    let absorption = wall_absorption(room, &wall);
    let diffusion = wall_diffusion(room, &wall);
    let spread = (diffusion * 200.0) as usize;

    let air = (-dist * 0.05).exp();
    let gain = (1.0 - absorption) * air * (1.0 / (1.0 + dist * dist));
    let delay = (dist / 343.0 * sample_rate) as usize;
    
    let main = Reflection { delay, gain };
    let diffuse = Reflection { delay: delay + spread, gain: main.gain * diffusion };

    WallReflection { primary: main, secondary: diffuse }
}
