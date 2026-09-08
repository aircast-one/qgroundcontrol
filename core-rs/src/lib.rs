#[cfg(not(test))]
mod abi;
mod altitude;
mod battery;
mod control;
mod guided;
mod instruments;
mod label;
mod links;
mod mapscale;
mod messages;
mod missionkinds;
mod plan;
mod preflight;
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
