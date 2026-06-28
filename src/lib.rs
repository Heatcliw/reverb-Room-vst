// src/lib.rs

pub mod roomsetting;
pub mod engine;

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState, egui::{self, Color32, Pos2, Sense, TextStyle, FontId, FontFamily}};
use std::sync::Arc;

use crate::roomsetting::{
    BoxRoom, DirectSound, Listener, MultiMaterial, Room, Source, 
    Vec3, Wall, WallReflection, Walls, wall_reflection, BRICK, WOOD, CONCRETE, DRYWALL
};
use crate::engine::TappedDelayEngine;




#[derive(Params)]
pub struct ReverbParams {
    #[id = "room_width"] pub width: FloatParam,
    #[id = "room_length"] pub length: FloatParam,
    #[id = "room_height"] pub height: FloatParam,
    
    // ========== МАТЕРИАЛЫ СТЕН ==========
    #[id = "wall_material"] pub wall_material: FloatParam,
    // 0.0 = Brick, 1.0 = Wood, 2.0 = Concrete, 3.0 = Drywall
    // ========== ПАРАМЕТРЫ ЭФФЕКТА ==========
    #[id = "wet_dry_mix"] pub wet_dry_mix: FloatParam,
    // Баланс сухой vs влажный звук (0-100%)
    #[id = "diffusion_amount"] pub diffusion_amount: FloatParam,
    // Количество диффузии (рассеивания)
    #[id = "reverb_time"] pub reverb_time: FloatParam,
    // RT60 - время затухания в секундах
    #[id = "early_reflections"] pub early_reflections: FloatParam,
    // Количество ранних отражений

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

            wall_material: FloatParam::new("Material", 2.0, FloatRange::Linear { min: 0.0, max: 3.0 }),
            wet_dry_mix: FloatParam::new("wet dry mix", 50.0, FloatRange::Linear { min: 0.0, max: 100.0 }),
            diffusion_amount: FloatParam::new("diffusion amount", 20.0, FloatRange::Linear { min: 0.0, max: 50.0 }),
            reverb_time: FloatParam::new("reverb time", 5.0, FloatRange::Linear { min: 0.0, max: 15.0 }),
            early_reflections: FloatParam::new("early reflections", 0.0, FloatRange::Linear { min: 0.0, max: 50.0 }),
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
            editor_state: EguiState::from_size(800, 500),
            engine: TappedDelayEngine::new(),
            last_width: 0.0, last_length: 0.0, last_height: 0.0,
            last_src_x: 0.0, last_src_y: 0.0, last_src_z: 0.0,
            last_lis_x: 0.0, last_lis_y: 0.0, last_lis_z: 0.0,
            sample_rate: 44100.0,
        }
    }
}

/// 1. Перечисление для типов виджетов
pub enum WidgetType {
    /// Обычный ползунок с указанием (Минимум, Максимум)
    Slider { min: f32, max: f32 },
    /// Логарифмический ползунок для частот (Минимум, Максимум)
    LogSlider { min: f32, max: f32 },
    /// Кнопка-переключатель (вкл/выкл)
    Toggle,
    /// Выпадающий список с вариантами текста
    ComboBox { options: Vec<&'static str> },
}

impl AcousticBoxReverb {
    /// 2. Универсальная функция отрисовки и привязки параметра
    pub fn draw_widget(
        ui: &mut egui::Ui,
        setter: &ParamSetter,
        param: &FloatParam,       // Наш плагин-параметр из ReverbParams
        label: &str,              // Название виджета на экране
        widget_type: WidgetType,  // Описанный выше тип
    ) {
        // Получаем текущее значение из DAW/плагина
        let mut value = param.value();
        let mut changed = false;

        // MATCH: Определяем, какой именно виджет рисовать на экране
        match widget_type {
            WidgetType::Slider { min, max } => {
                let res = ui.add(egui::Slider::new(&mut value, min..=max).text(label));
                changed = res.changed();
            }
            WidgetType::LogSlider { min, max } => {
                let res = ui.add(egui::Slider::new(&mut value, min..=max).text(label).logarithmic(true));
                changed = res.changed();
            }
            WidgetType::Toggle => {
                // Для кнопки переключателя: f32 > 0.5 — это истина (включено)
                let mut bool_val = value > 0.5;
                if ui.checkbox(&mut bool_val, label).changed() {
                    value = if bool_val { 1.0 } else { 0.0 };
                    changed = true;
                }
            }
            WidgetType::ComboBox { options } => {
                // Превращаем f32 индекс в целое число для выбора из списка
                let mut selected_idx = value.round() as usize;
                
                let res = egui::ComboBox::from_label(label)
                    .show_index(ui, &mut selected_idx, options.len(), |i| options[i]);
                
                if res.changed() {
                    value = selected_idx as f32;
                    changed = true;
                }
            }
        }

        // Блок автоматизации: Если значение изменилось, отправляем его в DAW
        if changed {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, value);
            setter.end_set_parameter(param);
        }
    }
     fn recalculate_acoustics(&mut self) {
        let w = self.params.width.value();
        let l = self.params.length.value();
        let h = self.params.height.value();

        // 1. Динамический выбор материала на основе параметров плагина
        let mat_idx = self.params.wall_material.value().round() as usize;
        let chosen_material = match mat_idx {
            0 => BRICK,
            1 => WOOD,
            2 => CONCRETE, // Раскомментируйте, если они есть в импортах roomsetting
            3 => DRYWALL,
            _ => BRICK,
        };

        // Исправлено: используем метод ::new() вместо фигурных скобок
        let multi_material = MultiMaterial::new(vec![(chosen_material, 1.0)]);
        
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

        // 2. Считываем углы поворота источника (Yaw / Pitch)
        let src_yaw = self.params.source_yaw.value();
        let src_pitch = self.params.source_pitch.value();

        let source = Source {
            position: Vec3 {
                x: self.params.source_x.value().min(w), 
                y: self.params.source_y.value().min(l),
                z: self.params.source_z.value().min(h),
            },
            yaw: src_yaw,
            pitch: src_pitch,
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
                // Избавляемся от лишнего match, копируя или клонируя enum
                let wall_enum = *wall; 
                
                // Если ваша функция wall_reflection умеет принимать углы или весь Source, 
                // передайте туда &source вместо &source.position:
                wall_reflection(&source, &listener.position, self.sample_rate, &box_room.structure, wall_enum)
            })
            .collect();

        self.engine.update_taps(&ds, &reflections);

        // Обновляем кэш предыдущих значений
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
            params.clone(), // Передаем параметры как состояние для egui

            |_state, _size| {}, 
            move |egui_ctx: &egui::Context, setter: &ParamSetter, _state: &mut Arc<ReverbParams>| {
                
                // 1. Создаем конфигурацию шрифтов по умолчанию
                let mut fonts = egui::FontDefinitions::default();

                // 2. Загружаем ваш файл японского шрифта прямо в бинарник плагина
                fonts.font_data.insert(
                    "japanese_font".to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(
                        include_bytes!("assets/NotoSansJP-Regular.otf").to_vec(),
                    )),
                );

                // 3. Ставим его на первое место для обычного (пропорционального) текста
                fonts.families.get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "japanese_font".to_owned());

                // 4. (Опционально) Ставим его также и для моноширинного текста, если он где-то нужен
                fonts.families.get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "japanese_font".to_owned());

                // 5. Применяем шрифты к контексту плагина
                egui_ctx.set_fonts(fonts);

                
                egui::CentralPanel::default().show(egui_ctx, |ui: &mut egui::Ui| {

                     ui.columns(2, |columns| {
                    let ui_left = &mut columns[0];
                    ui_left.heading("パラメーター");
                    // Добавляем ScrollArea, чтобы обилие слайдеров не ломало интерфейс по высоте
                    egui::ScrollArea::vertical()
                        .id_source("params_scroll")
                        .show(ui_left, |ui_scroll| {
                            
                            // ========== 部屋のサイズ (РАЗМЕРЫ КОМНАТЫ) ==========
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.width,
                                "幅 (m)", // Ширина
                                WidgetType::Slider { min: 1.0, max: 20.0 }
                            );
                            
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.length,
                                "奥行き (m)", // Длина / Глубина
                                WidgetType::Slider { min: 1.0, max: 20.0 }
                            );
                            
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.height,
                                "高さ (m)", // Высота
                                WidgetType::Slider { min: 1.0, max: 5.0 }
                            );
                            
                            // ========== 壁の材質 (ВЫБОР МАТЕРИАЛА) ==========
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.wall_material,
                                "壁の材質", // Материал стен
                                WidgetType::ComboBox { 
                                    options: vec!["レンガ (Brick)", "木材 (Wood)", "コンクリート (Concrete)", "石膏ボード (Drywall)"] 
                                }
                            );
                            
                            // ========== エフェクト設定 (ПАРАМЕТРЫ ЭФФЕКТА) ==========
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.wet_dry_mix,
                                "Wet/Dry ミックス (%)", // Wet/Dry Mix (%)
                                WidgetType::Slider { min: 0.0, max: 100.0 }
                            );

                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.diffusion_amount,
                                "ディフュージョン量", // Diffusion Amount
                                WidgetType::Slider { min: 0.0, max: 50.0 }
                            );

                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.reverb_time,
                                "リバーブタイム (RT60)", // Reverb Time (RT60)
                                WidgetType::Slider { min: 0.0, max: 15.0 }
                            );

                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.early_reflections,
                                "初期反射量", // Early Reflections
                                WidgetType::Slider { min: 0.0, max: 50.0 }
                            );
                            
                            // ========== 音源 (ИСТОЧНИК) ==========
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.source_x,
                                "音源位置 (0X)", 
                                WidgetType::Slider { min: 0.0, max: 20.0 }
                            );
                            
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.source_y,
                                "音源位置 (0Y)", 
                                WidgetType::Slider { min: 0.0, max: 20.0 }
                            );
                            
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.source_z,
                                "音源位置 (0Z)", 
                                WidgetType::Slider { min: 0.0, max: 5.0 }
                            );

                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.source_yaw,
                                "音源 - ヨー (回転)", // Источник - Yaw (Поворот)
                                WidgetType::Slider { min: 0.0, max: 360.0 }
                            );

                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.source_pitch,
                                "音源 - ピッチ (傾き)", // Источник - Pitch (Наклон)
                                WidgetType::Slider { min: -90.0, max: 90.0 }
                            );

                            // ========== リスナー (СЛУШАТЕЛЬ) ==========
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.listener_x,
                                "リスナー位置 (0X)", 
                                WidgetType::Slider { min: 0.0, max: 20.0 }
                            );
                            
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.listener_y,
                                "リスナー位置 (0Y)", 
                                WidgetType::Slider { min: 0.0, max: 20.0 }
                            );
                            
                            Self::draw_widget(
                                ui_scroll,
                                setter,
                                &params.listener_z,
                                "リスナー位置 (0Z)", 
                                WidgetType::Slider { min: 0.0, max: 5.0 }
                            );
                        });
                    ui_left.add_space(10.0);

                    ui_left.label(
                        egui::RichText::new("version 0.40b")
                            .size(11.0)
                            .color(egui::Color32::from_gray(120))
                    );
                    ui_left.separator();
                    ui_left.add_space(10.0);

                    let ui_right = &mut columns[1];
 
                    ui_right.add_space(10.0);
                    ui_right.heading(egui::RichText::new("ルーム・アコースティック・マップ").size(16.0));
                    ui_right.label(egui::RichText::new("ドットをドラッグ：赤 — 音源、青 — リスナー").size(13.0));
                    ui_right.separator();
                    
                    // ========== ОТРИСОВКА И ЛОГИКА КАРТЫ ==========
                    let canvas_size = 300.0;
                    let max_room_size = 20.0; // Максимальная граница FloatRange для нормализации сетки

                    let (rect, response): (egui::Rect, egui::Response) = ui_right.allocate_exact_size(
                        egui::vec2(canvas_size, canvas_size),
                        Sense::click_and_drag()
                    );
                    
                    // 1. Фон координатной сетки плагина
                    ui_right.painter().rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 20));
                    
                    let room_w = params.width.value();
                    let room_l = params.length.value();

                    let padding = 10.0;
                    let usable_canvas_size = canvas_size - (padding * 2.0);
                    let wall_start_pos = Pos2::new(rect.min.x + padding, rect.min.y + padding);
                    // 2. Вычисление и отрисовка динамических стен реальной комнаты
                    let room_screen_w = (room_w / max_room_size) * usable_canvas_size;
                    let room_screen_l = (room_l / max_room_size) * usable_canvas_size;
                    
                    let room_rect = egui::Rect::from_min_size(
                        wall_start_pos,
                        egui::vec2(room_screen_w, room_screen_l)
                    );

                    ui_right.painter().add(egui::Shape::rect_stroke(
                        room_rect,
                        0.0,
                        egui::Stroke::new(2.0, Color32::WHITE),
                        egui::StrokeKind::Outside
                    ));
                    
                    // 3. Расчет экранных позиций точек (с ограничением под размер стен комнаты)
                    let clamped_src_x = params.source_x.value().min(room_w);
                    let clamped_src_y = params.source_y.value().min(room_l);
                    let src_screen_x = room_rect.min.x + (clamped_src_x / room_w) * room_screen_w;
                    let src_screen_y = room_rect.max.y - (clamped_src_y / room_l) * room_screen_l;
                    let src_pos = Pos2::new(src_screen_x, src_screen_y);

                    let clamped_lis_x = params.listener_x.value().min(room_w);
                    let clamped_lis_y = params.listener_y.value().min(room_l);
                    let lis_screen_x = room_rect.min.x + (clamped_lis_x / room_w) * room_screen_w;
                    let lis_screen_y = room_rect.max.y - (clamped_lis_y / room_l) * room_screen_l;
                    let lis_pos = Pos2::new(lis_screen_x, lis_screen_y);

                    // Уникальный ID для хранения состояния захвата в egui
                    let drag_target_id = ui_right.id().with("drag_target");

                    // 4. НАЧАЛО КЛИКА МЫШИ
                    if response.drag_started() {
                        if let Some(mouse_pos) = response.interact_pointer_pos() {
                            let click_radius = 15.0; // Радиус захвата точки в пикселях
                            
                            if mouse_pos.distance(src_pos) < click_radius {
                                ui_right.data_mut(|d| d.insert_temp(drag_target_id, 1)); // 1 = Источник
                                setter.begin_set_parameter(&params.source_x);
                                setter.begin_set_parameter(&params.source_y);
                            } else if mouse_pos.distance(lis_pos) < click_radius {
                                ui_right.data_mut(|d| d.insert_temp(drag_target_id, 2)); // 2 = Слушатель
                                setter.begin_set_parameter(&params.listener_x);
                                setter.begin_set_parameter(&params.listener_y);
                            } else {
                                ui_right.data_mut(|d| d.insert_temp(drag_target_id, 0)); // Клик мимо
                            }
                        }
                    }

                    // 5. ДВИЖЕНИЕ МЫШИ (DRAG)
                    if response.dragged() {
                        let active_target: i32 = ui_right.data(|d| d.get_temp(drag_target_id).unwrap_or(0));

                        if active_target != 0 {
                            if let Some(mouse_pos) = response.interact_pointer_pos() {
                                // Переводим пиксели холста обратно в абсолютные метры
                                let norm_x = ((mouse_pos.x - rect.min.x) / canvas_size).clamp(0.0, 1.0);
                                let norm_y = ((mouse_pos.y - rect.min.y) / canvas_size).clamp(0.0, 1.0);
                                
                                let absolute_meter_x = norm_x * max_room_size;
                                let absolute_meter_y = norm_y * max_room_size;

                                // Точка не должна выходить за пределы текущей ширины и длины стен
                                let meter_x = absolute_meter_x.clamp(0.0, room_w);
                                let meter_y = absolute_meter_y.clamp(0.0, room_l);

                                if active_target == 1 {
                                    setter.set_parameter(&params.source_x, meter_x);
                                    setter.set_parameter(&params.source_y, meter_y);
                        } else if active_target == 2 {
                            setter.set_parameter(&params.listener_x, meter_x);
                            setter.set_parameter(&params.listener_y, meter_y);
                        }
                    }
                }
            }

            // 6. ОТПУСКАНИЕ МЫШИ
            if response.drag_stopped() {
                let active_target: i32 = ui_right.data(|d| d.get_temp(drag_target_id).unwrap_or(0));
                if active_target == 1 {
                    setter.end_set_parameter(&params.source_x);
                    setter.end_set_parameter(&params.source_y);
                } else if active_target == 2 {
                    setter.end_set_parameter(&params.listener_x);
                    setter.end_set_parameter(&params.listener_y);
                }
                ui_right.data_mut(|d| d.insert_temp(drag_target_id, 0)); // Очищаем захват
            }

            // 7. ОТРИСОВКА МАРКЕРОВ НА КАРТЕ
            // Источник (Красный)
            ui_right.painter().circle_filled(src_pos, 6.0, Color32::from_rgb(255, 50, 50));
            ui_right.painter().circle_stroke(src_pos, 8.0, egui::Stroke::new(1.0, Color32::WHITE));

            // Слушатель (Синий)
            ui_right.painter().circle_filled(lis_pos, 6.0, Color32::from_rgb(50, 100, 255));
            ui_right.painter().circle_stroke(lis_pos, 8.0, egui::Stroke::new(1.0, Color32::WHITE));

            // ========================================================
            // НОВОЕ: Отрисовка линий звука (Прямой луч и отражения)
            // ========================================================

            // Цвет для прямого звука (полупрозрачный белый/желтый)
            let direct_line_color = Color32::from_rgba_unmultiplied(255, 255, 200, 150);
            // Цвет для отражений (полупрозрачный оранжевый)
            let reflection_line_color = Color32::from_rgba_unmultiplied(230, 100, 50, 100);

            // 1. Отрисовка ПРЯМОГО звука (Линия между Источником и Слушателем)
            ui_right.painter().line_segment(
                [src_pos, lis_pos],
                egui::Stroke::new(1.5, direct_line_color)
            );

            // 2. Отрисовка РАННИХ ОТРАЖЕНИЙ от 4-х стен (Геометрический метод мнимых источников)
            // Отражение от ЛЕВОЙ стены
            let img_src_left = Pos2::new(room_rect.min.x - (src_pos.x - room_rect.min.x), src_pos.y);
            if let Some(hit_point) = intersect_lines(img_src_left, lis_pos, room_rect.min.x, true) {
                ui_right.painter().line_segment([src_pos, hit_point], egui::Stroke::new(1.0, reflection_line_color));
                ui_right.painter().line_segment([hit_point, lis_pos], egui::Stroke::new(1.0, reflection_line_color));
            }

            // Отражение от ПРАВОЙ стены
            let img_src_right = Pos2::new(room_rect.max.x + (room_rect.max.x - src_pos.x), src_pos.y);
            if let Some(hit_point) = intersect_lines(img_src_right, lis_pos, room_rect.max.x, true) {
                ui_right.painter().line_segment([src_pos, hit_point], egui::Stroke::new(1.0, reflection_line_color));
                ui_right.painter().line_segment([hit_point, lis_pos], egui::Stroke::new(1.0, reflection_line_color));
            }

            // Отражение от ВЕРХНЕЙ стены
            let img_src_top = Pos2::new(src_pos.x, room_rect.min.y - (src_pos.y - room_rect.min.y));
            if let Some(hit_point) = intersect_lines(img_src_top, lis_pos, room_rect.min.y, false) {
                ui_right.painter().line_segment([src_pos, hit_point], egui::Stroke::new(1.0, reflection_line_color));
                ui_right.painter().line_segment([hit_point, lis_pos], egui::Stroke::new(1.0, reflection_line_color));
            }

            // Отражение от НИЖНЕЙ стены
            let img_src_bottom = Pos2::new(src_pos.x, room_rect.max.y + (room_rect.max.y - src_pos.y));
            if let Some(hit_point) = intersect_lines(img_src_bottom, lis_pos, room_rect.max.y, false) {
                ui_right.painter().line_segment([src_pos, hit_point], egui::Stroke::new(1.0, reflection_line_color));
                ui_right.painter().line_segment([hit_point, lis_pos], egui::Stroke::new(1.0, reflection_line_color));
            }

            ui_right.add_space(10.0);
            });
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

// Вспомогательная функция для поиска точки отражения на стене
fn intersect_lines(p1: egui::Pos2, p2: egui::Pos2, wall_coord: f32, is_vertical_wall: bool) -> Option<egui::Pos2> {
    if (p2.x - p1.x).abs() < 0.0001 && is_vertical_wall { return None; }
    if (p2.y - p1.y).abs() < 0.0001 && !is_vertical_wall { return None; }

    if is_vertical_wall {
        let t = (wall_coord - p1.x) / (p2.x - p1.x);
        if (0.0..=1.0).contains(&t) {
            return Some(egui::Pos2::new(wall_coord, p1.y + t * (p2.y - p1.y)));
        }
    } else {
        let t = (wall_coord - p1.y) / (p2.y - p1.y);
        if (0.0..=1.0).contains(&t) {
            return Some(egui::Pos2::new(p1.x + t * (p2.x - p1.x), wall_coord));
        }
    }
    None
}



nih_export_vst3!(AcousticBoxReverb);
