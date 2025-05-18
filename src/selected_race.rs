use crate::json_value_from_bw;
use json::JsonValue;

#[derive(Debug, Copy, Clone)]
pub struct RequestRaceChange {
    pub id: u32,
    pub selected_race: u8,
}


pub fn parse_race_change(ptr: *mut crate::bw::JsonValue) -> Option<RequestRaceChange> {
    let json: JsonValue = unsafe { json_value_from_bw(ptr)? };

    if json["data"]["endpoint"].as_str()? != "RequestRaceChange" {
        return None;
    }

    let data = &json["data"]["data"];

    Some(RequestRaceChange {
        id: data["id"].as_u32()?,
        selected_race: data["selected_race"].as_u8()?,
    })
}