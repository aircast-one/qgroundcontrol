#[cfg(not(test))]
mod abi;
mod altitude;
mod battery;
mod calibration;
mod control;
mod guided;
mod inspector;
mod instruments;
mod label;
mod links;
mod logs;
mod mapscale;
mod messages;
mod missionkinds;
mod plan;
mod preflight;
mod radio;
mod read;
mod sensors;
mod speed;
mod takeoff;
mod terrain;
mod router;
mod vibration;
mod view;
mod warnings;

pub use router::{Backend, Core};
