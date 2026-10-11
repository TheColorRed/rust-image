mod add_noise;
mod despeckle;
mod median;

pub use add_noise::{Noise, NoiseDistribution, noise};
pub use despeckle::{Despeckle, despeckle};
pub use median::{Median, median};
