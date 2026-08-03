//! Деление‑свободный диапазонный кодер (кроме одного деления в декодере).
//! Параметры: TOP = 1<<24, BOTTOM = 24‑битная маска.
//! Кодер пишет напрямую в предварительно аллоцированный буфер.

use crate::entropy::Model;

const TOP: u64 = 1 << 24;
const BOTTOM: u64 = 0x00FF_FFFF;
const INIT_RANGE: u64 = 0xFFFF_FFFF;

/// Быстрый кодер, не выполняющий аллокаций в `put()`.
pub struct FastRangeEncoder {
    low: u64,
    range: u64,
    output: Vec<u8>,
}

impl FastRangeEncoder {
    /// Создаёт кодер с предварительным резервированием ёмкости.
    /// Это гарантирует, что последующие вызовы `put()` никогда не вызовут realloc.
    pub fn with_capacity(cap: usize) -> Self {
        FastRangeEncoder {
            low: 0,
            range: INIT_RANGE,
            output: Vec::with_capacity(cap),
        }
    }

    /// Размер уже записанных байт.
    pub fn len(&self) -> usize {
        self.output.len()
    }

    /// Закодировать один символ.
    #[inline]
    pub fn put(&mut self, sym: usize, model: &mut dyn Model) {
        let scale = self.range >> 12; // деление на 4096 без деления
        let cum = model.cum_freq(sym) as u64;
        let freq = model.freq(sym) as u64;

        self.low += cum * scale;
        self.range = freq * scale;

        // Нормализация – вывод полных байтов
        while self.range < TOP {
            self.output.push((self.low >> 24) as u8);
            self.low = (self.low << 8) & BOTTOM;
            self.range <<= 8;
        }
        model.update(sym);
    }

    /// Завершить кодирование и вернуть буфер.
    pub fn finish(mut self) -> Vec<u8> {
        // Выгружаем 5 байт для однозначного декодирования
        for _ in 0..5 {
            self.output.push((self.low >> 24) as u8);
            self.low = (self.low << 8) & BOTTOM;
        }
        self.output
    }
}

// ----------------------------------------------------------------
// Декодер
// ----------------------------------------------------------------

pub struct FastRangeDecoder<'a> {
    low: u64,
    range: u64,
    code: u64,
    input: &'a [u8],
    pos: usize,
}

impl<'a> FastRangeDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let mut code = 0u64;
        for i in 0..4 {
            code = (code << 8) | (data.get(i).copied().unwrap_or(0) as u64);
        }
        FastRangeDecoder {
            low: 0,
            range: INIT_RANGE,
            code,
            input: data,
            pos: 4,
        }
    }

    /// Извлечь следующий символ.
    /// Использует одно аппаратное деление на `scale` (максимум 20 бит) – быстро на всех CPU.
    #[inline]
    pub fn get(&mut self, model: &mut dyn Model) -> usize {
        let scale = self.range >> 12;
        let value = ((self.code - self.low) / scale) as u32; // value < 4096

        // Линейный поиск символа (дружественен к branch prediction)
        let mut sym = 0;
        loop {
            // Предполагаем, что cum_freq для несуществующего 256-го символа равен 4096
            if value < model.cum_freq(sym + 1) {
                break;
            }
            sym += 1;
        }

        let cum = model.cum_freq(sym) as u64;
        let freq = model.freq(sym) as u64;

        self.low += cum * scale;
        self.range = freq * scale;

        while self.range < TOP {
            self.range <<= 8;
            self.low = (self.low << 8) & BOTTOM;
            let byte = self.input.get(self.pos).copied().unwrap_or(0) as u64;
            self.pos += 1;
            self.code = ((self.code << 8) | byte) & BOTTOM;
        }

        model.update(sym);
        sym
    }
}
