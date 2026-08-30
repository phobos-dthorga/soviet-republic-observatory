use std::collections::BTreeMap;

use crate::compatibility_profile::{
    BinaryField, BinaryLayout, BinaryPrimitive, BinaryRecordCount, ByteOrder,
};
use crate::error::ObservatoryError;
use crate::model::BinaryMappedFact;

pub fn decode_layout(
    layout: &BinaryLayout,
    bytes: &[u8],
) -> Result<Vec<BinaryMappedFact>, ObservatoryError> {
    for check in &layout.magic_checks {
        let expected = decode_hex(&check.bytes_hex)?;
        let start = bounded_offset(check.offset, bytes.len())?;
        let end = start
            .checked_add(expected.len())
            .filter(|end| *end <= bytes.len())
            .ok_or(ObservatoryError::BinaryCompatibilityMismatch(
                "magic_out_of_range",
            ))?;
        if bytes[start..end] != expected {
            return Err(ObservatoryError::BinaryCompatibilityMismatch(
                "magic_mismatch",
            ));
        }
    }

    let count = match layout.record_count {
        BinaryRecordCount::Fixed { value } => usize::try_from(value)
            .map_err(|_| ObservatoryError::BinaryCompatibilityMismatch("record_count"))?,
        BinaryRecordCount::Field { offset, primitive } => {
            let start = bounded_offset(offset, bytes.len())?;
            let value = read_unsigned(bytes, start, primitive, layout.byte_order)?;
            usize::try_from(value)
                .map_err(|_| ObservatoryError::BinaryCompatibilityMismatch("record_count"))?
        }
    };
    if count == 0 || count > layout.max_records as usize {
        return Err(ObservatoryError::BinaryCompatibilityMismatch(
            "record_count",
        ));
    }
    let base = bounded_offset(layout.base_offset, bytes.len())?;
    let extent = count
        .checked_mul(layout.stride as usize)
        .and_then(|size| base.checked_add(size))
        .filter(|end| *end <= bytes.len())
        .ok_or(ObservatoryError::BinaryCompatibilityMismatch(
            "record_extent",
        ))?;
    let _ = extent;

    let mut facts = Vec::with_capacity(count.saturating_mul(layout.fields.len()));
    for record_index in 0..count {
        let record_start = base
            .checked_add(record_index.saturating_mul(layout.stride as usize))
            .ok_or(ObservatoryError::BinaryCompatibilityMismatch(
                "record_offset",
            ))?;
        let mut seen = BTreeMap::new();
        for field in &layout.fields {
            if seen.insert(&field.host_slot, ()).is_some() {
                return Err(ObservatoryError::BinaryCompatibilityMismatch(
                    "duplicate_slot",
                ));
            }
            let offset = record_start.checked_add(field.offset as usize).ok_or(
                ObservatoryError::BinaryCompatibilityMismatch("field_offset"),
            )?;
            let raw = read_field(bytes, offset, field, layout.byte_order)?;
            let available = !field.missing_values.contains(&raw);
            let value = if available {
                let scaled = raw * field.scale.unwrap_or(1.0);
                if !scaled.is_finite() {
                    return Err(ObservatoryError::BinaryCompatibilityMismatch(
                        "non_finite_value",
                    ));
                }
                Some(scaled)
            } else {
                None
            };
            facts.push(BinaryMappedFact {
                layout_id: layout.id.clone(),
                record_index: record_index.min(u32::MAX as usize) as u32,
                host_slot: field.host_slot.clone(),
                value,
                source_offset: offset.min(u64::MAX as usize) as u64,
            });
        }
    }
    Ok(facts)
}

fn read_field(
    bytes: &[u8],
    offset: usize,
    field: &BinaryField,
    order: ByteOrder,
) -> Result<f64, ObservatoryError> {
    if let Some(mask) = field.mask {
        return Ok((read_unsigned(bytes, offset, field.primitive, order)? & mask) as f64);
    }
    let slice = field_bytes(bytes, offset, field.primitive.size())?;
    let value = match (field.primitive, order) {
        (BinaryPrimitive::U8, _) => slice[0] as f64,
        (BinaryPrimitive::I8, _) => i8::from_ne_bytes([slice[0]]) as f64,
        (BinaryPrimitive::U16, ByteOrder::Little) => {
            u16::from_le_bytes(slice.try_into().expect("two bytes")) as f64
        }
        (BinaryPrimitive::U16, ByteOrder::Big) => {
            u16::from_be_bytes(slice.try_into().expect("two bytes")) as f64
        }
        (BinaryPrimitive::I16, ByteOrder::Little) => {
            i16::from_le_bytes(slice.try_into().expect("two bytes")) as f64
        }
        (BinaryPrimitive::I16, ByteOrder::Big) => {
            i16::from_be_bytes(slice.try_into().expect("two bytes")) as f64
        }
        (BinaryPrimitive::U32, ByteOrder::Little) => {
            u32::from_le_bytes(slice.try_into().expect("four bytes")) as f64
        }
        (BinaryPrimitive::U32, ByteOrder::Big) => {
            u32::from_be_bytes(slice.try_into().expect("four bytes")) as f64
        }
        (BinaryPrimitive::I32, ByteOrder::Little) => {
            i32::from_le_bytes(slice.try_into().expect("four bytes")) as f64
        }
        (BinaryPrimitive::I32, ByteOrder::Big) => {
            i32::from_be_bytes(slice.try_into().expect("four bytes")) as f64
        }
        (BinaryPrimitive::U64, ByteOrder::Little) => {
            u64::from_le_bytes(slice.try_into().expect("eight bytes")) as f64
        }
        (BinaryPrimitive::U64, ByteOrder::Big) => {
            u64::from_be_bytes(slice.try_into().expect("eight bytes")) as f64
        }
        (BinaryPrimitive::I64, ByteOrder::Little) => {
            i64::from_le_bytes(slice.try_into().expect("eight bytes")) as f64
        }
        (BinaryPrimitive::I64, ByteOrder::Big) => {
            i64::from_be_bytes(slice.try_into().expect("eight bytes")) as f64
        }
        (BinaryPrimitive::F32, ByteOrder::Little) => {
            f32::from_le_bytes(slice.try_into().expect("four bytes")) as f64
        }
        (BinaryPrimitive::F32, ByteOrder::Big) => {
            f32::from_be_bytes(slice.try_into().expect("four bytes")) as f64
        }
        (BinaryPrimitive::F64, ByteOrder::Little) => {
            f64::from_le_bytes(slice.try_into().expect("eight bytes"))
        }
        (BinaryPrimitive::F64, ByteOrder::Big) => {
            f64::from_be_bytes(slice.try_into().expect("eight bytes"))
        }
    };
    if !value.is_finite() {
        return Err(ObservatoryError::BinaryCompatibilityMismatch(
            "non_finite_value",
        ));
    }
    Ok(value)
}

fn read_unsigned(
    bytes: &[u8],
    offset: usize,
    primitive: BinaryPrimitive,
    order: ByteOrder,
) -> Result<u64, ObservatoryError> {
    let slice = field_bytes(bytes, offset, primitive.size())?;
    let value = match (primitive, order) {
        (BinaryPrimitive::U8 | BinaryPrimitive::I8, _) => u64::from(slice[0]),
        (BinaryPrimitive::U16 | BinaryPrimitive::I16, ByteOrder::Little) => {
            u64::from(u16::from_le_bytes(slice.try_into().expect("two bytes")))
        }
        (BinaryPrimitive::U16 | BinaryPrimitive::I16, ByteOrder::Big) => {
            u64::from(u16::from_be_bytes(slice.try_into().expect("two bytes")))
        }
        (BinaryPrimitive::U32 | BinaryPrimitive::I32, ByteOrder::Little) => {
            u64::from(u32::from_le_bytes(slice.try_into().expect("four bytes")))
        }
        (BinaryPrimitive::U32 | BinaryPrimitive::I32, ByteOrder::Big) => {
            u64::from(u32::from_be_bytes(slice.try_into().expect("four bytes")))
        }
        (BinaryPrimitive::U64 | BinaryPrimitive::I64, ByteOrder::Little) => {
            u64::from_le_bytes(slice.try_into().expect("eight bytes"))
        }
        (BinaryPrimitive::U64 | BinaryPrimitive::I64, ByteOrder::Big) => {
            u64::from_be_bytes(slice.try_into().expect("eight bytes"))
        }
        (BinaryPrimitive::F32 | BinaryPrimitive::F64, _) => {
            return Err(ObservatoryError::BinaryCompatibilityMismatch(
                "integer_primitive_required",
            ));
        }
    };
    Ok(value)
}

fn field_bytes(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8], ObservatoryError> {
    let end = offset
        .checked_add(size)
        .filter(|end| *end <= bytes.len())
        .ok_or(ObservatoryError::BinaryCompatibilityMismatch(
            "truncated_field",
        ))?;
    Ok(&bytes[offset..end])
}

fn bounded_offset(offset: u64, length: usize) -> Result<usize, ObservatoryError> {
    usize::try_from(offset)
        .ok()
        .filter(|offset| *offset <= length)
        .ok_or(ObservatoryError::BinaryCompatibilityMismatch(
            "offset_out_of_range",
        ))
}

fn decode_hex(value: &str) -> Result<Vec<u8>, ObservatoryError> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| ObservatoryError::BinaryCompatibilityMismatch("invalid_magic_hex"))?;
            u8::from_str_radix(text, 16)
                .map_err(|_| ObservatoryError::BinaryCompatibilityMismatch("invalid_magic_hex"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::decode_layout;
    use crate::compatibility_profile::{
        BinaryField, BinaryLayout, BinaryMagicCheck, BinaryPrimitive, BinaryRecordCount, ByteOrder,
    };

    #[test]
    fn decodes_every_allowed_primitive_with_masks_scaling_and_sentinels() {
        let mut bytes = vec![0x52, 0x4f];
        bytes.extend_from_slice(&[
            250,
            (-5_i8) as u8,
            0x34,
            0x12,
            0xfe,
            0xff,
            0x78,
            0x56,
            0x34,
            0x12,
            0xfc,
            0xff,
            0xff,
            0xff,
        ]);
        bytes.extend_from_slice(&123_u64.to_le_bytes());
        bytes.extend_from_slice(&(-123_i64).to_le_bytes());
        bytes.extend_from_slice(&1.5_f32.to_le_bytes());
        bytes.extend_from_slice(&2.5_f64.to_le_bytes());
        let slots = [
            "core.citizens.electronics.none",
            "core.citizens.electronics.radio",
            "core.citizens.electronics.television",
            "core.citizens.electronics.computer",
            "source.stats.citizens.born",
            "source.stats.citizens.dead",
            "source.stats.citizens.escaped",
            "source.stats.citizens.immigrant_soviet",
            "source.stats.citizens.immigrant_africa",
            "source.stats.citizens.small_children",
        ];
        let primitives = [
            BinaryPrimitive::U8,
            BinaryPrimitive::I8,
            BinaryPrimitive::U16,
            BinaryPrimitive::I16,
            BinaryPrimitive::U32,
            BinaryPrimitive::I32,
            BinaryPrimitive::U64,
            BinaryPrimitive::I64,
            BinaryPrimitive::F32,
            BinaryPrimitive::F64,
        ];
        let mut offset = 0_u32;
        let fields = slots
            .iter()
            .zip(primitives)
            .enumerate()
            .map(|(index, (slot, primitive))| {
                let field = BinaryField {
                    host_slot: (*slot).to_owned(),
                    offset,
                    primitive,
                    mask: (index == 0).then_some(0x0f),
                    scale: (index == 8).then_some(2.0),
                    missing_values: if index == 6 { vec![123.0] } else { vec![] },
                };
                offset += primitive.size() as u32;
                field
            })
            .collect();
        let layout = BinaryLayout {
            id: "fixture.all-primitives".to_owned(),
            entry_name: "fixture.bin".to_owned(),
            byte_order: ByteOrder::Little,
            base_offset: 2,
            record_count: BinaryRecordCount::Fixed { value: 1 },
            stride: offset,
            max_records: 1,
            magic_checks: vec![BinaryMagicCheck {
                offset: 0,
                bytes_hex: "524f".to_owned(),
            }],
            fields,
        };
        let facts = decode_layout(&layout, &bytes).expect("decoded facts");
        assert_eq!(facts.len(), 10);
        assert_eq!(facts[0].value, Some(10.0));
        assert_eq!(facts[1].value, Some(-5.0));
        assert_eq!(facts[6].value, None);
        assert_eq!(facts[8].value, Some(3.0));
        assert_eq!(facts[9].value, Some(2.5));
    }

    #[test]
    fn rejects_magic_mismatch_truncation_overflow_and_non_finite_values() {
        let layout = BinaryLayout {
            id: "fixture.failure".to_owned(),
            entry_name: "fixture.bin".to_owned(),
            byte_order: ByteOrder::Big,
            base_offset: 0,
            record_count: BinaryRecordCount::Fixed { value: 2 },
            stride: 8,
            max_records: 2,
            magic_checks: vec![BinaryMagicCheck {
                offset: 0,
                bytes_hex: "abcd".to_owned(),
            }],
            fields: vec![BinaryField {
                host_slot: "core.citizens.electronics.none".to_owned(),
                offset: 0,
                primitive: BinaryPrimitive::F64,
                mask: None,
                scale: None,
                missing_values: vec![],
            }],
        };
        assert!(decode_layout(&layout, &[0; 16]).is_err());
        let mut non_finite = f64::NAN.to_be_bytes().to_vec();
        non_finite.extend_from_slice(&[0; 8]);
        let mut without_magic = layout;
        without_magic.magic_checks.clear();
        assert!(decode_layout(&without_magic, &non_finite).is_err());
        assert!(decode_layout(&without_magic, &[0; 8]).is_err());
    }
}
