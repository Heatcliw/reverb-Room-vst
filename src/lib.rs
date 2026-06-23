// src/lib.rs

pub mod roomsetting;
pub mod engine;

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState, egui::{self, Color32, Pos2, Sense}};
use std::sync::Arc;

use crate::roomsetting::{
    BoxRoom, DirectSound, Listener, MultiMaterial, Room, Source, 
    Vec3, Wall, WallReflection, Walls, wall_reflection, BRICK, WOOD
};
use crate::engine::TappedDelayEngine;

#[derive(Params)]
pub struct ReverbParams {
    #[id = "room_width"] pub width: FloatParam,
    #[id = "room_length"] pub length: FloatParam,
    #[id = "room_height"] pub height: FloatParam,

    #[id = "source_x"] pub source_x: FloatParam,
    #[id = "source_y"] pub source_y: FloatParam,
    #[id = "source_z"] pub source_z: FloatParam,
    #[id = "source_yaw"] pub source_yaw: FloatParam,
    #[id = "source_pitch"] pub source_pitch: FloatParam,

    #[id = "listener_x"] pub listener_x: FloatParam,
    #[id = "listener_y"] pub listener_y: FloatParam,
    #[id = "listener_z"] pub listener_z: FloatParam,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            width: FloatParam::new("Room Width", 10.0, FloatRange::Linear { min: 1.0, max: 20.0 }),
            length: FloatParam::new("Room Length", 10.0, FloatRange::Linear { min: 1.0, max: 20.0 }),
            height: FloatParam::new("Room Height", 2.5, FloatRange::Linear { min: 1.0, max: 5.0 }),

            source_x: FloatParam::new("Source X", 3.0, FloatRange::Linear { min: 0.0, max: 20.0 }),
            source_y: FloatParam::new("Source Y", 2.0, FloatRange::Linear { min: 0.0, max: 20.0 }),
            source_z: FloatParam::new("Source Z", 1.5, FloatRange::Linear { min: 0.0, max: 5.0 }),
            source_yaw: FloatParam::new("Source Yaw", 45.0, FloatRange::Linear { min: 0.0, max: 360.0 }),
            source_pitch: FloatParam::new("Source Pitch", 0.0, FloatRange::Linear { min: -90.0, max: 90.0 }),

            listener_x: FloatParam::new("Listener X", 7.0, FloatRange::Linear { min: 0.0, max: 20.0 }),
            listener_y: FloatParam::new("Listener Y", 6.0, FloatRange::Linear { min: 0.0, max: 20.0 }),
            listener_z: FloatParam::new("Listener Z", 1.5, FloatRange::Linear { min: 0.0, max: 5.0 }),
        }
    }
}

pub struct AcousticBoxReverb {
    params: Arc<ReverbParams>,
    editor_state: Arc<EguiState>,
    engine: TappedDelayEngine,
    last_width: f32, last_length: f32, last_height: f32,
    last_src_x: f32, last_src_y: f32, last_src_z: f32,
    last_lis_x: f32, last_lis_y: f32, last_lis_z: f32,
    sample_rate: f32,
}

impl Default for AcousticBoxReverb {
    fn default() -> Self {
        Self {
            params: Arc::new(ReverbParams::default()),
            editor_state: EguiState::from_size(340, 400),
            engine: TappedDelayEngine::new(),
            last_width: 0.0, last_length: 0.0, last_height: 0.0,
            last_src_x: 0.0, last_src_y: 0.0, last_src_z: 0.0,
            last_lis_x: 0.0, last_lis_y: 0.0, last_lis_z: 0.0,
            sample_rate: 44100.0,
        }
    }
}

impl AcousticBoxReverb {
    fn recalculate_acoustics(&mut self) {
        let w = self.params.width.value();
        let l = self.params.length.value();
        let h = self.params.height.value();

        let multi_material = MultiMaterial { materials: vec![(BRICK, 0.7), (WOOD, 0.3)] };
        
        let box_room = BoxRoom {
            structure: Room {
                width: w, length: l, height: h,
                walls: Walls {
                    left_wall: multi_material.clone(), right_wall: multi_material.clone(),
                    front_wall: multi_material.clone(), back_wall: multi_material.clone(),
                    floor: multi_material.clone(), ceiling: multi_material.clone(),
                }
            }
        };

        let source = Source {
            position: Vec3 {
                x: self.params.source_x.value().min(w), 
                y: self.params.source_y.value().min(l),
                z: self.params.source_z.value().min(h),
            },
            yaw: self.params.source_yaw.value(),
            pitch: self.params.source_pitch.value(),
        };

        let listener = Listener {
            position: Vec3 {
                x: self.params.listener_x.value().min(w),
                y: self.params.listener_y.value().min(l),
                z: self.params.listener_z.value().min(h),
            },
        };

        let ds = DirectSound::from(&source.position, &listener.position, self.sample_rate);

        let walls = [Wall::Left, Wall::Right, Wall::Front, Wall::Back, Wall::Floor, Wall::Ceiling];
        let reflections: Vec<WallReflection> = walls.iter()
            .map(|wall| {
                let wall_enum = match wall {
                    Wall::Left => Wall::Left, Wall::Right => Wall::Right,
                    Wall::Front => Wall::Front, Wall::Back => Wall::Back,
                    Wall::Floor => Wall::Floor, Wall::Ceiling => Wall::Ceiling,
                };
                wall_reflection(&source.position, &listener.position, self.sample_rate, &box_room.structure, wall_enum)
            })
            .collect();

        self.engine.update_taps(&ds, &reflections);

        self.last_width = w; self.last_length = l; self.last_height = h;
        self.last_src_x = source.position.x; self.last_src_y = source.position.y; self.last_src_z = source.position.z;
        self.last_lis_x = listener.position.x; self.last_lis_y = listener.position.y; self.last_lis_z = listener.position.z;
    }
}

impl Plugin for AcousticBoxReverb {
    const NAME: &'static str = "Visual Box Reverb";
    const VENDOR: &'static str = "DSP Engineer";
    const URL: &'static str = "https://example.com";
    const EMAIL: &'static str = "dsp@example.com";
    const VERSION: &'static str = "1.0.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: std::num::NonZeroU32::new(1),  
            main_output_channels: std::num::NonZeroU32::new(1), 
            ..AudioIOLayout::const_default()
        }
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

        fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        
        create_egui_editor(
            self.editor_state.clone(),
            params.clone(),

            |_, _| {}, 
            // ИСПРАВЛЕНО: Третий аргумент теперь строго соответствует ожидаемому типу &mut Arc<ReverbParams>
            move |egui_ctx: &egui::Context, setter: &ParamSetter, _state: &mut Arc<ReverbParams>| {
                egui::CentralPanel::default().show(egui_ctx, |ui: &mut egui::Ui| {
                    ui.heading("Акустическая Карта Комнаты");
                    ui.label("Двигайте точки: Красная - Источник, Синяя - Слушатель");

                    let canvas_size = 300.0;
                    let (rect, response): (egui::Rect, egui::Response) = ui.allocate_exact_size(
                        egui::vec2(canvas_size, canvas_size),
                        Sense::click_and_drag()
                    );

                    ui.painter().add(egui::Shape::rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(2.0, Color32::WHITE),
                        egui::StrokeKind::Outside
                    ));

                    let room_w = params.width.value();
                    let room_l = params.length.value();

                    if response.dragged() {
                        if let Some(mouse_pos) = response.interact_pointer_pos() {
                            let norm_x = ((mouse_pos.x - rect.min.x) / canvas_size).clamp(0.0, 1.0);
                            let norm_y = ((mouse_pos.y - rect.min.y) / canvas_size).clamp(0.0, 1.0);
                            
                            let meter_x = norm_x * room_w;
                            let meter_y = norm_y * room_l;

                            let src_pos = Pos2::new(
                                rect.min.x + (params.source_x.value() / room_w) * canvas_size,
                                rect.min.y + (params.source_y.value() / room_l) * canvas_size,
                            );
                            
                            if mouse_pos.distance(src_pos) < canvas_size * 0.1 {
                                setter.set_parameter(&params.source_x, meter_x);
                                setter.set_parameter(&params.source_y, meter_y);
                            } else {
                                setter.set_parameter(&params.listener_x, meter_x);
                                setter.set_parameter(&params.listener_y, meter_y);
                            }
                        }
                    }

                    let src_screen_x = rect.min.x + (params.source_x.value() / room_w) * canvas_size;
                    let src_screen_y = rect.min.y + (params.source_y.value() / room_l) * canvas_size;
                    
                    let lis_screen_x = rect.min.x + (params.listener_x.value() / room_w) * canvas_size;
                    let lis_screen_y = rect.min.y + (params.listener_y.value() / room_l) * canvas_size;

                    ui.painter().circle_filled(
                        Pos2::new(src_screen_x, src_screen_y), 
                        8.0, 
                        Color32::from_rgb(255, 50, 50)
                    );

                    ui.painter().circle_filled(
                        Pos2::new(lis_screen_x, lis_screen_y), 
                        8.0, 
                        Color32::from_rgb(50, 50, 255)
                    );
                });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layouts: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.engine.clear();
        self.recalculate_acoustics();
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if (self.params.width.value() - self.last_width).abs() > 0.001
            || (self.params.length.value() - self.last_length).abs() > 0.001
            || (self.params.height.value() - self.last_height).abs() > 0.001
            || (self.params.source_x.value() - self.last_src_x).abs() > 0.001
            || (self.params.source_y.value() - self.last_src_y).abs() > 0.001
            || (self.params.source_z.value() - self.last_src_z).abs() > 0.001
            || (self.params.listener_x.value() - self.last_lis_x).abs() > 0.001
            || (self.params.listener_y.value() - self.last_lis_y).abs() > 0.001
            || (self.params.listener_z.value() - self.last_lis_z).abs() > 0.001
        {
            self.recalculate_acoustics();
        }

        for mut block_samples in buffer.iter_samples() {
            let input_sample = *block_samples.get_mut(0).unwrap();
            let output_sample = self.engine.process_sample(input_sample);

            for sample in block_samples.iter_mut() {
                *sample = output_sample;
            }
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for AcousticBoxReverb {
    const VST3_CLASS_ID: [u8; 16] = *b"BoxRoomVisualDSP";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Reverb,
    ];
}

nih_export_vst3!(AcousticBoxReverb);
