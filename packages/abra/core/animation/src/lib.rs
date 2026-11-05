//! Things that change over time. Where a thing is at a moment of its life, worked out for thousands of things at once, so that a caller (an
//! effect, a preview, a program on a server or code across a language boundary) does not pay for each one by itself.
//!
//! Nothing here knows about pictures: it works out numbers. What draws them lives in the crates that draw.

mod ensemble;
mod impulse;
mod motion;
mod schedule;

pub use ensemble::{Ensemble, EnsembleOptions, PoseNoise, SpeedResponse, Frame, HISTORY_POINT_VALUES, HistoryRun, LIFETIME_STEPS, MEMBER_VALUES, POSE_VALUES, PoseRun};
pub use impulse::Impulse;
pub use motion::{MOVER_VALUES, Motion, Mover, MoverPicks};
pub use schedule::Schedule;
