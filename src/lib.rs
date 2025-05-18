#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;

use json::JsonValue;
use samase_plugin::{FuncId, PluginApi};


pub mod bw {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union JsonValue {
        pub integer: i64,
        pub string: JsonString,
        pub inline_string: JsonInlineString,
        pub object: JsonObject,
        pub type_flags: JsonTypeFlags,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct JsonString {
        pub len: u32,
        pub data: *mut u8,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct JsonInlineString {
        pub data: [u8; 0xd],
        pub length: u8, // 0xd - length so can be used as null term
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct JsonObject {
        pub entries: u32,
        pub capacity: u32,
        pub kv_pairs: *mut JsonValue,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct JsonTypeFlags {
        pub _x: [u8; 0xe],
        pub value: u16,
    }
}
pub mod sr;

unsafe fn json_value_from_bw(bw: *mut bw::JsonValue) -> Option<JsonValue> {
    match (*bw).type_flags.value & 0x7 {
        1 => Some(JsonValue::Boolean(true)),
        2 => Some(JsonValue::Boolean(false)),
        3 => {
            let len = (*bw).object.entries as usize;
            let ptr = (*bw).object.kv_pairs.map_addr(|x| x & 0x0000_ffff_ffff_ffff);
            let mut out = json::object::Object::with_capacity(len);
            for i in 0..len {
                let key = json_value_from_bw(ptr.add(i * 2))?;
                let value = json_value_from_bw(ptr.add(i * 2 + 1))?;
                out.insert(key.as_str().unwrap_or("??"), value);
            }
            Some(JsonValue::Object(out))
        }
        4 => {
            let len = (*bw).object.entries as usize;
            let ptr = (*bw).object.kv_pairs.map_addr(|x| x & 0x0000_ffff_ffff_ffff);
            let mut out = Vec::new();
            for i in 0..len {
                out.push(json_value_from_bw(ptr.add(i))?);
            }
            Some(JsonValue::Array(out))
        }
        5 => {
            let (ptr, len) = if (*bw).type_flags.value & 0x1000 != 0 {
                (
                    &raw const (*bw).inline_string.data as *const u8,
                    0xd - (*bw).inline_string.length as usize,
                )
            } else {
                (
                    (*bw).string.data.cast_const().map_addr(|x| x & 0x0000_ffff_ffff_ffff),
                    (*bw).string.len as usize,
                )
            };
            let slice = std::slice::from_raw_parts(ptr, len);
            Some(JsonValue::String(String::from_utf8_lossy(slice).into()))
        }
        6 => Some(JsonValue::Number((*bw).integer.into())),
        _ => None,
    }
}



#[unsafe(no_mangle)]
pub unsafe extern "C" fn samase_plugin_init(api: *const PluginApi) {
    let _ = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}:{}][{}] {}",
                record.file().unwrap_or("???"),
                record.line().unwrap_or(0),
                record.level(),
                message))
        })
        .level(log::LevelFilter::Trace)
        .chain(fern::log_file("LOBBY TEST LOG.log").unwrap())
        .apply();
    log::info!("START");

    plugin_api_hook(api, FuncId::LobbyScreenOnWebUiMessage, lobby_webui_hook as usize);
}

unsafe extern "C" fn lobby_webui_hook(
    this: *mut c_void,
    json: *mut bw::JsonValue,
    orig: unsafe extern "C" fn (*mut c_void, *mut bw::JsonValue) -> usize,
) -> usize {
    if let Some(json) = json_value_from_bw(json) {
        log::debug!("LOBBY MESSAGE {}", json.pretty(2));
    } else {
        log::debug!("FAILED TO PARSE LOBBY MESSAGE");
    }
    orig(this, json)
}

unsafe fn plugin_api_hook(
    api: *const PluginApi,
    func: FuncId,
    f: usize,
) {
    let ok = ((*api).hook_func)(func as u16, f);
    if ok == 0 {
        panic!("Failed to hook func {func:?}");
    }
}



