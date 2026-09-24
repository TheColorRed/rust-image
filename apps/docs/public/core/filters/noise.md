---
title: Noise filters
order: 4
outline: deep
---

# Noise filters

Noise filters add texture or remove isolated pixel variation. They preserve the alpha channel while processing RGB data. See [Apply options](../apply-options) for regional processing.

## Add noise

`noise` adds deterministic per-pixel noise using a uniform or Gaussian distribution:

```rust
use abra::filters::prelude::noise::{NoiseDistribution, noise};

noise(12.0).apply(&mut image);
noise(12.0).with_distribution(NoiseDistribution::Gaussian).apply(&mut image);
```

`with_distribution` sets the distribution, which defaults to `Uniform`:

- `NoiseDistribution::Uniform`: even probability across the noise range.
- `NoiseDistribution::Gaussian`: normal distribution generated with Box-Muller sampling.

The amount controls the strength of the RGB perturbation. The alpha channel is preserved.

## Median filter

`median` replaces each channel with the local median, which can reduce impulsive noise while retaining stronger edges:

```rust
use abra::filters::prelude::noise::median;

median(&mut image, 2.0, None);
```

The radius is rounded to a non-negative integer. Small radii use a sliding histogram approach; larger radii use a downsampled approximation for performance.

## Despeckle

`despeckle` removes isolated outlier pixels while attempting to preserve edges:

```rust
use abra::filters::prelude::noise::despeckle;

despeckle(&mut image, 1.0, 13.0, None);
```

The arguments are neighborhood radius and channel-difference threshold. The filter compares a pixel against local medians and replaces strong outliers.
