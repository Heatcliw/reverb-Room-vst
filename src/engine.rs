// src/engine.rs

use crate::roomsetting::{DirectSound, WallReflection};

pub struct TappedDelayEngine {
    buffer: Vec<f32>,
    write_ptr: usize,
    // Набор пар: (задержка в сэмплах, коэффициент усиления)
    taps: Vec<(usize, f32)>,
}

impl TappedDelayEngine {
    pub fn new() -> Self {
        // Создаем фиксированный буфер на ~3 секунды при 48kHz,
        // чтобы избежать выделения памяти на куче во время обработки звука.
        Self {
            buffer: vec![0.0; 144000],
            write_ptr: 0,
            taps: Vec::new(),
        }
    }

    // Очистка истории при сбросе или старте плагина
    pub fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.write_ptr = 0;
    }

    // Обновление карты задержек на основе расчетов геометрии
    pub fn update_taps(&mut self, ds: &DirectSound, reflections: &[WallReflection]) {
        self.taps.clear();
        
        // Добавляем прямой звук
        self.taps.push((ds.delay, ds.gain));
        
        // Добавляем первичные и диффузные отражения от всех стен
        for wr in reflections {
            self.taps.push((wr.primary.delay, wr.primary.gain));
            self.taps.push((wr.secondary.delay, wr.secondary.gain));
        }
    }

    // Посэмпл-обработка (чтение из прошлого на основе кольцевого индекса)
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        self.buffer[self.write_ptr] = input;

        let mut output = 0.0;
        let buf_len = self.buffer.len();

        for &(delay, gain) in &self.taps {
            if delay < buf_len {
                let read_ptr = (self.write_ptr + buf_len - delay) % buf_len;
                output += self.buffer[read_ptr] * gain;
            }
        }

        self.write_ptr = (self.write_ptr + 1) % buf_len;
        output
    }
}
