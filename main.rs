

// pub mod roomsetting;

// use crate::roomsetting::{BoxRoom, DirectSound, Listener, Material, MultiMaterial, Reflection, Room, RoomGeometry, Source, Vec3, Wall, WallReflection, Walls, distance, process, wall_reflection};

// pub const CONCRETE: Material = Material {
//     absorption: 0.02,
//     diffusion: 0.10,
// };

// pub const BRICK: Material = Material {
//     absorption: 0.03,
//     diffusion: 0.20,
// };

// pub const CLAY: Material = Material {
//     absorption: 0.05,
//     diffusion: 0.15,
// };

// pub const WOOD: Material = Material {
//     absorption: 0.15,
//     diffusion: 0.25,
// };

// pub const CARPET: Material = Material {
//     absorption: 0.40,
//     diffusion: 0.30,
// };

// fn main () {

//     let multi_material = MultiMaterial {
//         materials: vec![
//             (BRICK, 0.7),
//             (WOOD, 0.3),
//         ]
//     };

//     let room = BoxRoom {
//         structure: Room
//         {
//             width: 2.5,
//             length: 2.5,
//             height: 2.5,

//             walls: Walls {
//                 left_wall: multi_material.clone(),
//                 right_wall: multi_material.clone(),
//                 front_wall: multi_material.clone(),
//                 back_wall: multi_material.clone(),
//                 floor: multi_material.clone(),
//                 ceiling: multi_material.clone(),
//             }
//         }
//     };

//     let volume = room.volume();

//     let position1 = Vec3 {
//         x: 3.0,
//         y: 2.0,
//         z: 4.0,
//     };

//     let position2 = Vec3 {
//         x: 2.0,
//         y: 2.0,
//         z: 8.0,
//     };

//     let source = Source {
//         position: position1,
//         yaw: 45.0,
//         pitch: 0.0,
//     };

//     let listener = Listener {
//         position: position2,
//     };
    
//     let dist = distance(&source.position, &listener.position);

//     let sample_rate = 48000.0;
//     let size = 48000; // 1 секунда

//     let mut input = vec![0.0; size];
//     let mut output = vec![0.0; size];

//     let ds = DirectSound::from(&source.position, &listener.position, sample_rate);
//     let reflections = vec![
//         wall_reflection(&source.position, &listener.position, sample_rate, &room.structure, Wall::Back),
//         wall_reflection(&source.position, &listener.position, sample_rate, &room.structure, Wall::Right),
//         wall_reflection(&source.position, &listener.position, sample_rate, &room.structure, Wall::Left),
//         wall_reflection(&source.position, &listener.position, sample_rate, &room.structure, Wall::Floor),
//         wall_reflection(&source.position, &listener.position, sample_rate, &room.structure, Wall::Front),
//         wall_reflection(&source.position, &listener.position, sample_rate, &room.structure, Wall::Ceiling)
//     ];

//     output.fill(0.0);
//     process(&input, &mut output, ds, reflections);

//     export_wav(&output, sample_rate);
//     println!("Material: [{}, {}], width: [{}], length: [{}], height: [{}], volume: [{}], distance: [{}]", room.structure.walls.floor.absorption(), room.structure.walls.floor.diffusion(), room.structure.width, room.structure.length, room.structure.height, volume, dist);
// }

// fn export_wav(output: &Vec<f32>, sample_rate: f32) {
//     use hound;

//     let spec = hound::WavSpec {
//         channels: 1,
//         sample_rate: sample_rate as u32,
//         bits_per_sample: 32,
//         sample_format: hound::SampleFormat::Float,
//     };

//     let mut writer = hound::WavWriter::create("test.wav", spec).unwrap();

//     for sample in output {
//         writer.write_sample(*sample).unwrap();
//     }

//     writer.finalize().unwrap();
// }