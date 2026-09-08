#[cfg(not(test))]
mod abi;
mod altitude;
mod battery;
mod control;
mod guided;
mod instruments;
mod label;
mod links;
mod messages;
mod plan;
mod preflight;
mod read;
mod sensors;
mod speed;
mod takeoff;
mod router;
mod vibration;
mod view;
mod warnings;

pub use router::{Backend, Core};
