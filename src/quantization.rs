use std::num::NonZeroU16;

/// Quantization table used for encoding
#[derive(Debug, Clone)]
pub enum QuantizationTableType {
    /// Sample quantization tables given in Annex K (Clause K.1) of Recommendation ITU-T T.81 (1992) | ISO/IEC 10918-1:1994.
    Default,

    /// A user supplied quantization table
    Custom(Box<[u8; 64]>),
}

static DEFAULT_LUMA_TABLE: [u8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61,
    12, 12, 14, 19, 26, 58, 60, 55,
    14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62,
    18, 22, 37, 56, 68, 109, 103, 77,
    24, 35, 55, 64, 81, 104, 113, 92,
    49, 64, 78, 87, 103, 121, 120, 101,
    72, 92, 95, 98, 112, 100, 103, 99,
];

static DEFAULT_CHROMA_TABLE: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99,
    18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
];

pub struct QuantizationTable {
    table: [NonZeroU16; 64],
}

impl QuantizationTable {
    pub fn new_with_quality(table: &QuantizationTableType, quality: u8, luma: bool) -> QuantizationTable {
        let table = match table {
            QuantizationTableType::Custom(table) => Self::get_user_table(table),
            QuantizationTableType::Default => {
                let table = if luma {
                    &DEFAULT_LUMA_TABLE
                } else {
                    &DEFAULT_CHROMA_TABLE
                };
                Self::get_with_quality(table, quality)
            }
        };

        QuantizationTable {
            table,
        }
    }

    fn get_user_table(table: &[u8; 64]) -> [NonZeroU16; 64] {
        let mut q_table = [NonZeroU16::new(1).unwrap(); 64];
        for (i, &v) in table.iter().enumerate() {
            let v = v.max(1) as u16;
            q_table[i] = NonZeroU16::new(v << 3).unwrap();
        }
        q_table
    }

    fn get_with_quality(table: &[u8; 64], quality: u8) -> [NonZeroU16; 64] {
        let quality = quality.max(1).min(100) as u32;

        let scale = if quality < 50 {
            5000 / quality
        } else {
            200 - quality * 2
        };

        let mut q_table = [NonZeroU16::new(1).unwrap(); 64];

        for (i, &v) in table.iter().enumerate() {
            let v = v as u32;

            let v = (v * scale + 50) / 100;

            let v = v.max(1).min(255) as u16;

            // Table values are premultiplied with 8 because dct is scaled by 8
            q_table[i] = NonZeroU16::new(v << 3).unwrap();
        }
        q_table
    }


    #[inline]
    pub fn get(&self, index: usize) -> u8 {
        (self.table[index].get() >> 3) as u8
    }

    #[inline]
    pub fn quantize(&self, value: i16, index: usize) -> i16 {
        // Using i32 as intermediate value allows the compiler to remove an overflow check
        let q_value = self.table[index].get() as i32;

        let value = if value < 0 {
            let value = -value;
            let value = (value as i32 + (q_value / 2)) / q_value;
            -value
        } else {
            (value as i32 + (q_value / 2)) / q_value
        };

        value as i16
    }
}

#[cfg(test)]
mod tests {
    use crate::quantization::{QuantizationTable, QuantizationTableType};

    #[test]
    fn test_new_100() {
        let q = QuantizationTable::new_with_quality(&QuantizationTableType::Default, 100, true);

        for &v in &q.table {
            let v = v.get();
            assert_eq!(v, 1 << 3);
        }

        let q = QuantizationTable::new_with_quality(&QuantizationTableType::Default, 100, false);

        for &v in &q.table {
            let v = v.get();
            assert_eq!(v, 1 << 3);
        }
    }

    #[test]
    fn test_new_100_quantize() {
        let q = QuantizationTable::new_with_quality(&QuantizationTableType::Default, 100, true);

        for i in -255..255 {
            assert_eq!(i, q.quantize(i << 3, 0));
        }
    }
}