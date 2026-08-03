//! Сверхлёгкая адаптивная модель с фиксированной суммой 4096.
//! Кумулятивные частоты поддерживаются инкрементально – обновление за O(256-sym),
//! полное перестроение только при периодическом масштабировании.

/// Трейт модели вероятностей для диапазонного кодера.
pub trait Model {
    /// Всегда 4096.
    fn total(&self) -> u32;
    fn freq(&self, sym: usize) -> u32;
    fn cum_freq(&self, sym: usize) -> u32;
    fn update(&mut self, sym: usize);
    fn num_symbols(&self) -> usize;
}

/// Модель 256 символов, сумма всегда 4096.
pub struct FastAdaptiveModel {
    freq: [u32; 256],
    cum:  [u32; 256], // cum[i] = sum_{j<i} freq[j]
    total: u32,       // реальная сумма freq, не превышает 4096
}

impl FastAdaptiveModel {
    pub fn new() -> Self {
        let initial = 16u32; // 4096 / 256
        let mut freq = [initial; 256];
        let mut cum = [0u32; 256];
        let mut acc = 0;
        for i in 0..256 {
            cum[i] = acc;
            acc += freq[i];
        }
        FastAdaptiveModel { freq, cum, total: 4096 }
    }
}

impl Model for FastAdaptiveModel {
    #[inline(always)]
    fn total(&self) -> u32 { 4096 }

    #[inline(always)]
    fn freq(&self, sym: usize) -> u32 {
        unsafe { *self.freq.get_unchecked(sym) }
    }

    #[inline(always)]
    fn cum_freq(&self, sym: usize) -> u32 {
        unsafe { *self.cum.get_unchecked(sym) }
    }

    fn update(&mut self, sym: usize) {
        // Инкрементальное обновление кумулятивных частот
        self.freq[sym] += 1;
        self.total += 1;

        // Прибавляем 1 ко всем cum[k] для k > sym
        for k in (sym + 1)..256 {
            self.cum[k] += 1;
        }

        // Масштабирование при переполнении
        if self.total > 4096 {
            for f in &mut self.freq {
                *f = (*f >> 1) + 1; // никогда не обнуляется
            }
            // Полное перестроение cum и total (один проход)
            let mut acc = 0;
            for i in 0..256 {
                self.cum[i] = acc;
                acc += self.freq[i];
            }
            self.total = acc;
        }
    }

    #[inline(always)]
    fn num_symbols(&self) -> usize { 256 }
}
