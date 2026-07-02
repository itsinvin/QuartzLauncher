use std::sync::Arc;

use rc_zip_sync::{ArchiveHandle, HasCursor};
use rustc_hash::FxHashSet;
use serde::Deserialize;

const MIXIN_ANNOTATION: &str = "Lorg/spongepowered/asm/mixin/Mixin;";

#[derive(Deserialize)]
struct FabricModJsonMixins {
    mixins: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct MixinConfigJson {
    package: Option<String>,
    mixins: Option<Vec<serde_json::Value>>,
    client: Option<Vec<serde_json::Value>>,
    server: Option<Vec<serde_json::Value>>,
}

pub fn extract_mixin_targets<R: HasCursor>(
    archive: &ArchiveHandle<R>,
    fabric_mod_json: &[u8],
) -> Arc<[Arc<str>]> {
    let mut config_paths = FxHashSet::default();

    if let Ok(fabric) = serde_json::from_slice::<FabricModJsonMixins>(fabric_mod_json) {
        if let Some(mixins) = fabric.mixins {
            for entry in mixins {
                if let Some(path) = entry.as_str() {
                    config_paths.insert(path.to_string());
                }
            }
        }
    }

    for entry in archive.entries() {
        if entry.kind() != rc_zip_sync::rc_zip::EntryKind::File {
            continue;
        }
        if entry.name.ends_with(".mixins.json") {
            config_paths.insert(entry.name.to_string());
        }
    }

    let mut mixin_classes = FxHashSet::default();
    for config_path in config_paths {
        let Some(file) = archive.by_name(&config_path) else {
            continue;
        };
        let Ok(bytes) = file.bytes() else {
            continue;
        };
        collect_mixin_classes_from_config(&bytes, &mut mixin_classes);
    }

    let mut targets = FxHashSet::default();
    for class_name in mixin_classes {
        let class_path = format!("{class_name}.class");
        let Some(file) = archive.by_name(&class_path) else {
            continue;
        };
        let Ok(bytes) = file.bytes() else {
            continue;
        };
        collect_mixin_targets_from_class(&bytes, &mut targets);
    }

    let mut sorted: Vec<Arc<str>> = targets.into_iter().map(Arc::from).collect();
    sorted.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
    sorted.into()
}

fn collect_mixin_classes_from_config(bytes: &[u8], out: &mut FxHashSet<String>) {
    let Ok(config) = serde_json::from_slice::<MixinConfigJson>(bytes) else {
        return;
    };

    let package = config.package.as_deref().unwrap_or("").trim_matches('.');
    let package_prefix = if package.is_empty() {
        String::new()
    } else {
        format!("{package}/")
    };

    for list in [config.mixins, config.client, config.server].into_iter().flatten() {
        for entry in list {
            let Some(simple_name) = mixin_entry_name(&entry) else {
                continue;
            };
            if simple_name.contains('/') || simple_name.contains('.') && !simple_name.contains('$') {
                let class_name = simple_name.replace('.', "/");
                out.insert(class_name);
            } else {
                out.insert(format!("{package_prefix}{simple_name}"));
            }
        }
    }
}

fn mixin_entry_name(entry: &serde_json::Value) -> Option<&str> {
    match entry {
        serde_json::Value::String(name) => Some(name.as_str()),
        serde_json::Value::Object(map) => map
            .get("name")
            .or_else(|| map.get("config"))
            .and_then(|value| value.as_str()),
        _ => None,
    }
}

fn collect_mixin_targets_from_class(bytes: &[u8], out: &mut FxHashSet<String>) {
    let Some(pool) = ConstantPool::parse(bytes) else {
        return;
    };

    let Some(annotation_values) = pool.runtime_visible_class_annotations(bytes) else {
        return;
    };

    for value in annotation_values {
        if value == MIXIN_ANNOTATION {
            continue;
        }
        if let Some(class_name) = descriptor_to_class_name(&value) {
            out.insert(class_name);
        }
    }
}

fn descriptor_to_class_name(descriptor: &str) -> Option<String> {
    let descriptor = descriptor.strip_prefix('L')?.strip_suffix(';')?;
    if descriptor.is_empty() {
        return None;
    }
    Some(descriptor.replace('/', "."))
}

struct ConstantPool<'a> {
    strings: Vec<&'a str>,
}

impl<'a> ConstantPool<'a> {
    fn parse(bytes: &'a [u8]) -> Option<Self> {
        if bytes.len() < 10 || bytes.get(0..4) != Some(&[0xCA, 0xFE, 0xBA, 0xBE]) {
            return None;
        }

        let mut offset = 8_usize;
        let count = read_u16(bytes, &mut offset)? as usize;
        let mut strings = vec![""; count];

        let mut index = 1;
        while index < count {
            let tag = *bytes.get(offset)?;
            offset += 1;
            match tag {
                1 => {
                    let len = read_u16(bytes, &mut offset)? as usize;
                    let value = std::str::from_utf8(bytes.get(offset..offset + len)?).unwrap_or("");
                    strings[index] = value;
                    offset += len;
                },
                7 | 8 | 16 | 19 | 20 => offset += 2,
                3 | 4 | 9 | 10 | 11 | 17 | 18 => offset += 4,
                5 | 6 => {
                    offset += 8;
                    index += 1;
                },
                15 => offset += 3,
                _ => return None,
            }
            index += 1;
        }

        Some(Self { strings })
    }

    fn utf8(&self, index: u16) -> Option<&str> {
        self.strings.get(index as usize).copied()
    }

    fn runtime_visible_class_annotations(&self, bytes: &[u8]) -> Option<Vec<String>> {
        let mut offset = 8_usize;
        let count = read_u16(bytes, &mut offset)? as usize;
        let mut index = 1;
        while index < count {
            let tag = *bytes.get(offset)?;
            offset += 1;
            match tag {
                1 => {
                    let len = read_u16(bytes, &mut offset)? as usize;
                    offset += len;
                },
                7 | 8 | 16 | 19 | 20 => offset += 2,
                3 | 4 | 9 | 10 | 11 | 17 | 18 => offset += 4,
                5 | 6 => {
                    offset += 8;
                    index += 1;
                },
                15 => offset += 3,
                _ => return None,
            }
            index += 1;
        }

        offset += 6; // access_flags, this_class, super_class
        let interfaces_count = read_u16(bytes, &mut offset)? as usize;
        offset += interfaces_count * 2;

        offset = skip_fields(bytes, offset)?;
        offset = skip_methods(bytes, offset)?;

        let attributes_count = read_u16(bytes, &mut offset)? as usize;
        for _ in 0..attributes_count {
            let name_index = read_u16(bytes, &mut offset)?;
            let length = read_u32(bytes, &mut offset)? as usize;
            let name = self.utf8(name_index)?;
            if name == "RuntimeVisibleAnnotations" {
                return Some(parse_annotation_values(bytes, offset, length, self));
            }
            offset += length;
        }

        None
    }
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> Option<u16> {
    let value = bytes.get(*offset..*offset + 2)?;
    *offset += 2;
    Some(u16::from_be_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Option<u32> {
    let value = bytes.get(*offset..*offset + 4)?;
    *offset += 4;
    Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

fn skip_fields(bytes: &[u8], mut offset: usize) -> Option<usize> {
    let count = read_u16(bytes, &mut offset)? as usize;
    for _ in 0..count {
        offset += 6;
        offset = skip_attributes(bytes, offset)?;
    }
    Some(offset)
}

fn skip_methods(bytes: &[u8], mut offset: usize) -> Option<usize> {
    let count = read_u16(bytes, &mut offset)? as usize;
    for _ in 0..count {
        offset += 6;
        offset = skip_attributes(bytes, offset)?;
    }
    Some(offset)
}

fn skip_attributes(bytes: &[u8], mut offset: usize) -> Option<usize> {
    let count = read_u16(bytes, &mut offset)? as usize;
    for _ in 0..count {
        offset += 2;
        let length = read_u32(bytes, &mut offset)? as usize;
        offset += length;
    }
    Some(offset)
}

fn parse_annotation_values(
    bytes: &[u8],
    mut offset: usize,
    length: usize,
    pool: &ConstantPool<'_>,
) -> Vec<String> {
    let end = offset + length;
    let mut values = Vec::new();
    let Some(num_annotations) = read_u16(bytes, &mut offset) else {
        return values;
    };

    for _ in 0..num_annotations {
        let type_index = read_u16(bytes, &mut offset).unwrap_or(0);
        if pool.utf8(type_index) == Some(MIXIN_ANNOTATION) {
            values.push(MIXIN_ANNOTATION.to_string());
        }

        let pairs = read_u16(bytes, &mut offset).unwrap_or(0);
        for _ in 0..pairs {
            offset += 2;
            parse_element_value(bytes, &mut offset, end, pool, &mut values);
        }
    }

    values
}

fn parse_element_value(
    bytes: &[u8],
    offset: &mut usize,
    end: usize,
    pool: &ConstantPool<'_>,
    values: &mut Vec<String>,
) {
    if *offset >= end {
        return;
    }

    let tag = *bytes.get(*offset).unwrap_or(&0);
    *offset += 1;
    match tag {
        b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's' => {
            *offset += 2;
        },
        b'c' => {
            let const_index = read_u16(bytes, offset).unwrap_or(0);
            if let Some(value) = pool.utf8(const_index) {
                values.push(value.to_string());
            }
        },
        b'e' => {
            *offset += 4;
        },
        b'@' => {
            let type_index = read_u16(bytes, offset).unwrap_or(0);
            if pool.utf8(type_index) == Some(MIXIN_ANNOTATION) {
                values.push(MIXIN_ANNOTATION.to_string());
            }
            let pairs = read_u16(bytes, offset).unwrap_or(0);
            for _ in 0..pairs {
                *offset += 2;
                parse_element_value(bytes, offset, end, pool, values);
            }
        },
        b'[' => {
            let count = read_u16(bytes, offset).unwrap_or(0);
            for _ in 0..count {
                parse_element_value(bytes, offset, end, pool, values);
            }
        },
        _ => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_to_class_name_works() {
        assert_eq!(
            descriptor_to_class_name("Lnet/minecraft/class_437;"),
            Some("net.minecraft.class_437".into())
        );
    }
}
