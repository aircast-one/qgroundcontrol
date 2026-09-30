use regex::RegexBuilder;
use serde_json::Value;

pub const BUNDLED: &str = include_str!("../../src/Comms/USBBoardInfo.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardType {
    Pixhawk,
    SiKRadio,
    OpenPilot,
    RtkGps,
}

impl BoardType {
    pub fn name(self) -> &'static str {
        match self {
            BoardType::Pixhawk => "Pixhawk",
            BoardType::SiKRadio => "SiK Radio",
            BoardType::OpenPilot => "OpenPilot",
            BoardType::RtkGps => "RTK GPS",
        }
    }

    fn from_class(class: &str) -> Option<BoardType> {
        [BoardType::Pixhawk, BoardType::RtkGps, BoardType::SiKRadio, BoardType::OpenPilot].into_iter().find(|b| b.name() == class)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fallback {
    pub pattern: String,
    pub board: BoardType,
    pub android_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoardTable {
    pub boards: Vec<(u16, u16, BoardType, String)>,
    pub by_description: Vec<Fallback>,
    pub by_manufacturer: Vec<Fallback>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PortInfo {
    pub system_location: String,
    pub port_name: String,
    pub description: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
}

fn fallbacks(root: &Value, key: &str) -> Result<Vec<Fallback>, String> {
    root.get(key)
        .and_then(Value::as_array)
        .ok_or(format!("no {key} array"))?
        .iter()
        .map(|f| {
            let pattern = f.get("regExp").and_then(Value::as_str).ok_or("fallback without regExp")?;
            let board = BoardType::from_class(f.get("boardClass").and_then(Value::as_str).unwrap_or("")).ok_or("Bad board class")?;
            Ok(Fallback { pattern: pattern.to_string(), board, android_only: f.get("androidOnly").and_then(Value::as_bool).unwrap_or(false) })
        })
        .collect()
}

pub fn parse(text: &str) -> Result<BoardTable, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    if root.get("fileType").and_then(Value::as_str) != Some("USBBoardInfo") || root.get("version").and_then(Value::as_i64) != Some(1) {
        return Err("not a USBBoardInfo version 1 file".to_string());
    }
    let boards = root
        .get("boardInfo")
        .and_then(Value::as_array)
        .ok_or("no boardInfo array")?
        .iter()
        .map(|b| {
            let vid = b.get("vendorID").and_then(Value::as_u64).ok_or("board without vendorID")? as u16;
            let pid = b.get("productID").and_then(Value::as_u64).ok_or("board without productID")? as u16;
            let board = BoardType::from_class(b.get("boardClass").and_then(Value::as_str).unwrap_or("")).ok_or("Bad board class")?;
            Ok((vid, pid, board, b.get("name").and_then(Value::as_str).unwrap_or("").to_string()))
        })
        .collect::<Result<_, String>>()?;
    Ok(BoardTable { boards, by_description: fallbacks(&root, "boardDescriptionFallback")?, by_manufacturer: fallbacks(&root, "boardManufacturerFallback")? })
}

fn matches(pattern: &str, text: &str) -> bool {
    RegexBuilder::new(pattern).case_insensitive(true).build().is_ok_and(|r| r.is_match(text))
}

impl BoardTable {
    pub fn bundled() -> Result<BoardTable, String> {
        parse(BUNDLED)
    }

    pub fn classify(&self, port: &PortInfo, android: bool) -> Option<(BoardType, String)> {
        let by_id = port.vendor_id.zip(port.product_id).and_then(|(vid, pid)| self.boards.iter().find(|(v, p, _, _)| *v == vid && (*p == pid || *p == 0))).map(|(_, _, b, name)| (*b, name.clone()));
        let by_text = |list: &[Fallback], text: &str| list.iter().filter(|f| android || !f.android_only).find(|f| matches(&f.pattern, text)).map(|f| (f.board, f.board.name().to_string()));
        by_id.or_else(|| by_text(&self.by_description, &port.description)).or_else(|| by_text(&self.by_manufacturer, &port.manufacturer))
    }

    pub fn is_bootloader(&self, port: &PortInfo, android: bool) -> bool {
        self.classify(port, android).is_some_and(|(b, _)| b == BoardType::Pixhawk) && port.description.contains("BL")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(vid: u16, pid: u16, description: &str, manufacturer: &str) -> PortInfo {
        PortInfo { system_location: "/dev/cu.usbmodem1".into(), port_name: "cu.usbmodem1".into(), description: description.into(), manufacturer: manufacturer.into(), serial_number: "1".into(), vendor_id: Some(vid), product_id: Some(pid) }
    }

    #[test]
    fn the_bundled_table_classifies_by_id_then_description_then_manufacturer() {
        let table = BoardTable::bundled().unwrap();
        assert!(table.boards.len() > 40 && !table.by_description.is_empty() && !table.by_manufacturer.is_empty());
        assert_eq!(table.classify(&port(9900, 17, "", ""), false), Some((BoardType::Pixhawk, "PX4 FMU V2".into())));
        assert_eq!(table.classify(&port(1, 1, "px4 fmu v6u.x", ""), false), Some((BoardType::Pixhawk, "Pixhawk".into())));
        assert_eq!(table.classify(&port(1, 1, "", "ArduPilot"), false).map(|(b, _)| b), Some(BoardType::Pixhawk));
        assert_eq!(table.classify(&port(1, 1, "nothing", "nobody"), false), None);
        let bl = port(9900, 17, "PX4 BL FMU v2.x", "");
        assert!(table.is_bootloader(&bl, false) && !table.is_bootloader(&port(9900, 17, "PX4 FMU v2.x", ""), false));
    }

    #[test]
    fn android_only_fallbacks_apply_only_on_android_and_bad_files_are_refused() {
        let text = r#"{"fileType":"USBBoardInfo","version":1,"boardInfo":[{"vendorID":1,"productID":0,"boardClass":"SiK Radio","name":"Any SiK"}],"boardDescriptionFallback":[{"regExp":"^silabs","boardClass":"SiK Radio","androidOnly":true}],"boardManufacturerFallback":[]}"#;
        let table = parse(text).unwrap();
        assert_eq!(table.classify(&port(1, 77, "", ""), false), Some((BoardType::SiKRadio, "Any SiK".into())));
        let silabs = port(2, 2, "SiLabs CP2102", "");
        assert_eq!(table.classify(&silabs, false), None);
        assert_eq!(table.classify(&silabs, true), Some((BoardType::SiKRadio, "SiK Radio".into())));
        assert!(parse(r#"{"fileType":"USBBoardInfo","version":1,"boardInfo":[{"vendorID":1,"productID":1,"boardClass":"Toaster","name":"x"}],"boardDescriptionFallback":[],"boardManufacturerFallback":[]}"#).is_err());
        assert!(parse(r#"{"fileType":"Other","version":1}"#).is_err());
    }
}
