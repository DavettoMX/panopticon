use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read};

#[derive(Debug, Clone, Serialize)]
pub struct GgufMetadata {
    pub architecture: String,
    pub quantization: String,
    pub parameter_count: u64,
    pub layer_count: u32,
    pub embedding_length: u32,
    pub context_length: u32,
    pub head_count: u32,
    pub head_count_kv: u32,
    pub vocab_size: u32,
    pub file_type: u32,
    pub tensor_count: u64,
}

#[derive(Debug, Clone)]
enum GgufValue {
    Uint8(u8),
    Int8(i8),
    Uint16(u16),
    Int16(i16),
    Uint32(u32),
    Int32(i32),
    Float32(f32),
    Bool(bool),
    Str(String),
    Uint64(u64),
    Int64(i64),
    Float64(f64),
    Array(Vec<GgufValue>),
}

impl GgufValue {
    fn as_u32(&self) -> Option<u32> {
        match self {
            GgufValue::Uint32(v) => Some(*v),
            GgufValue::Int32(v) => Some(*v as u32),
            GgufValue::Uint64(v) => Some(*v as u32),
            GgufValue::Int64(v) => Some(*v as u32),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            GgufValue::Uint64(v) => Some(*v),
            GgufValue::Uint32(v) => Some(*v as u64),
            GgufValue::Int64(v) => Some(*v as u64),
            GgufValue::Int32(v) => Some(*v as u64),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            GgufValue::Str(s) => Some(s),
            _ => None,
        }
    }
}

fn read_u8(r: &mut impl Read) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_i8(r: &mut impl Read) -> io::Result<i8> {
    Ok(read_u8(r)? as i8)
}

fn read_u16(r: &mut impl Read) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_i16(r: &mut impl Read) -> io::Result<i16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i32(r: &mut impl Read) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_f32(r: &mut impl Read) -> io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_u64(r: &mut impl Read) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_i64(r: &mut impl Read) -> io::Result<i64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

fn read_f64(r: &mut impl Read) -> io::Result<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

fn read_string(r: &mut impl Read) -> io::Result<String> {
    let len = read_u64(r)? as usize;
    if len > 1_000_000 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "string too long"));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn read_value(r: &mut impl Read, vtype: u32) -> io::Result<GgufValue> {
    match vtype {
        0 => Ok(GgufValue::Uint8(read_u8(r)?)),
        1 => Ok(GgufValue::Int8(read_i8(r)?)),
        2 => Ok(GgufValue::Uint16(read_u16(r)?)),
        3 => Ok(GgufValue::Int16(read_i16(r)?)),
        4 => Ok(GgufValue::Uint32(read_u32(r)?)),
        5 => Ok(GgufValue::Int32(read_i32(r)?)),
        6 => Ok(GgufValue::Float32(read_f32(r)?)),
        7 => Ok(GgufValue::Bool(read_u8(r)? != 0)),
        8 => Ok(GgufValue::Str(read_string(r)?)),
        9 => {
            let arr_type = read_u32(r)?;
            let arr_len = read_u64(r)? as usize;
            if arr_len > 10_000_000 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "array too large"));
            }
            let mut arr = Vec::with_capacity(arr_len.min(1024));
            for _ in 0..arr_len {
                arr.push(read_value(r, arr_type)?);
            }
            Ok(GgufValue::Array(arr))
        }
        10 => Ok(GgufValue::Uint64(read_u64(r)?)),
        11 => Ok(GgufValue::Int64(read_i64(r)?)),
        12 => Ok(GgufValue::Float64(read_f64(r)?)),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown GGUF value type: {}", vtype))),
    }
}

fn file_type_name(ft: u32) -> &'static str {
    match ft {
        0 => "F32",
        1 => "F16",
        2 => "Q4_0",
        3 => "Q4_1",
        7 => "Q8_0",
        8 => "Q8_1",
        9 => "Q2_K",
        10 => "Q3_K_S",
        11 => "Q3_K_M",
        12 => "Q3_K_L",
        13 => "Q4_K_S",
        14 => "Q4_K_M",
        15 => "Q5_K_S",
        16 => "Q5_K_M",
        17 => "Q6_K",
        18 => "IQ2_XXS",
        19 => "IQ2_XS",
        20 => "IQ3_XXS",
        21 => "IQ1_S",
        22 => "IQ4_NL",
        23 => "IQ3_S",
        24 => "IQ2_S",
        25 => "IQ4_XS",
        26 => "IQ1_M",
        27 => "BF16",
        28 => "Q4_0_4_4",
        29 => "Q4_0_4_8",
        30 => "Q4_0_8_8",
        _ => "Unknown",
    }
}

pub fn read_gguf_metadata(path: &str) -> Result<GgufMetadata, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).map_err(|e| format!("Failed to read magic: {}", e))?;
    if &magic != b"GGUF" {
        return Err("Not a GGUF file".to_string());
    }

    let version = read_u32(&mut reader).map_err(|e| format!("Failed to read version: {}", e))?;
    if version < 2 || version > 3 {
        return Err(format!("Unsupported GGUF version: {}", version));
    }

    let tensor_count = read_u64(&mut reader).map_err(|e| e.to_string())?;
    let kv_count = read_u64(&mut reader).map_err(|e| e.to_string())?;

    let mut metadata: HashMap<String, GgufValue> = HashMap::new();
    for _ in 0..kv_count {
        let key = read_string(&mut reader).map_err(|e| format!("Failed to read key: {}", e))?;
        let vtype = read_u32(&mut reader).map_err(|e| format!("Failed to read value type: {}", e))?;
        let value = read_value(&mut reader, vtype).map_err(|e| format!("Failed to read value for '{}': {}", key, e))?;
        metadata.insert(key, value);
    }

    let architecture = metadata.get("general.architecture")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let file_type = metadata.get("general.file_type")
        .and_then(|v| v.as_u32())
        .unwrap_or(0);

    let quantization = file_type_name(file_type).to_string();

    let arch = &architecture;
    let layer_count = metadata.get(&format!("{}.block_count", arch))
        .or_else(|| metadata.get("general.block_count"))
        .and_then(|v| v.as_u32())
        .unwrap_or(0);

    let embedding_length = metadata.get(&format!("{}.embedding_length", arch))
        .and_then(|v| v.as_u32())
        .unwrap_or(0);

    let context_length = metadata.get(&format!("{}.context_length", arch))
        .and_then(|v| v.as_u32())
        .unwrap_or(0);

    let head_count = metadata.get(&format!("{}.attention.head_count", arch))
        .and_then(|v| v.as_u32())
        .unwrap_or(0);

    let head_count_kv = metadata.get(&format!("{}.attention.head_count_kv", arch))
        .and_then(|v| v.as_u32())
        .unwrap_or(0);

    let vocab_size = metadata.get(&format!("{}.vocab_size", arch))
        .and_then(|v| v.as_u32())
        .or_else(|| {
            metadata.get("tokenizer.ggml.tokens").and_then(|v| {
                if let GgufValue::Array(arr) = v {
                    Some(arr.len() as u32)
                } else {
                    None
                }
            })
        })
        .unwrap_or(0);

    let parameter_count = metadata.get("general.parameter_count")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| estimate_params(layer_count, embedding_length, vocab_size, head_count_kv, head_count));

    Ok(GgufMetadata {
        architecture,
        quantization,
        parameter_count,
        layer_count,
        embedding_length,
        context_length,
        head_count,
        head_count_kv,
        vocab_size,
        file_type,
        tensor_count,
    })
}

fn estimate_params(layers: u32, embed: u32, vocab: u32, kv_heads: u32, heads: u32) -> u64 {
    if layers == 0 || embed == 0 {
        return 0;
    }
    let e = embed as u64;
    let l = layers as u64;
    let v = vocab as u64;
    let h = heads.max(1) as u64;
    let kv_h = kv_heads.max(1) as u64;
    let head_dim = e / h;

    let attn_params = e * e + e * (kv_h * head_dim) * 2 + e * e;
    let ffn_dim = ((e * 8 / 3 + 127) / 128) * 128;
    let ffn_params = e * ffn_dim * 3;
    let norm_params = e * 2;

    let per_layer = attn_params + ffn_params + norm_params;
    let embedding_params = v * e;
    let final_norm = e;

    l * per_layer + embedding_params + final_norm
}
