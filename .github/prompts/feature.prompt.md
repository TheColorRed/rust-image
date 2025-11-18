---
description: Creates a new feature for the abra library.
---

<!--
Feature Creation Prompt Guidance
This file defines the decision framework and authoring template for proposing new capabilities.
Follow naming conventions in `.github/instructions/naming-conventions.instructions.md` for any public API additions.
-->

# Feature Creation

Use this document when proposing or implementing new functionality. Every idea must pass through a clear decision path: Core Library (abra) vs Plugin.

---
## 1. Goals

We optimize for:
1. Breadth: Increase generally useful image manipulation primitives (filters, color transforms, geometry, compositing, blending, masking).
2. Composability: New capabilities should layer cleanly with existing pipeline constructs (layers, masks, transforms, effects).
3. Maintainability: Keep core lean; push niche orchestration to plugins.
4. Performance: Favor implementations that can leverage SIMD, GPU paths, or avoid redundant passes.
5. Consistency: Align API naming, parameter semantics, and file/module placement with existing design.

---
## 2. Decision Tree (Core vs Plugin)

Answer these in order:
1. Does the idea introduce a NEW primitive image manipulation technique (novel algorithm, pixel operation, color math, sampling strategy)?
	 - YES → Core (abra).
	 - NO → Continue.
2. Can the idea be fully expressed as a composition of existing abra primitives (layers, existing filters, transforms, blends, masks)?
	 - YES → Plugin.
	 - NO → Continue.
3. Is the technique broadly applicable across multiple domains (photo editing, illustration, compositing, effects, batch processing)?
	 - YES → Core.
	 - NO → Plugin.
4. Would adding it to core reduce duplication or unlock multiple future features (acts as enabling primitive)?
	 - YES → Core.
	 - NO → Plugin.

Shortcut Summary:
- New pixel math / sampling / color space logic → Core.
- Pure orchestration or preset arrangements → Plugin.
- Highly themed / stylistic outcomes (e.g. seasonal collage, poster style) → Plugin.
- General transform (e.g. perspective warp) → Core.

---
## 3. Core Addition Criteria Checklist

All MUST be satisfied for direct inclusion:
- General applicability across workflows.
- Clear primitive boundary (single responsibility, not an ad-hoc pipeline bundle).
- Parameter set is minimal, explicit, and extensible.
- Composable with layers, masks, blends, and transforms without special cases.
- Implementation does not duplicate an existing primitive.
- Performance cost justified (benchmarks or estimated complexity vs alternatives).

If any fail → Re-scope or move to plugin.

---
## 4. Plugin Criteria Checklist

Recommended when ANY apply:
- Feature is a preset sequence (pipeline orchestration, style recipe, layout engine).
- Niche domain context (e.g. trading card layout, meme generator, scrapbook logic).
- Experimental algorithm not yet tuned for performance/stability.
- Optional heavy dependency (large lookup tables, fonts, theme packs) not needed by core.
- Rapid iteration expected (likely to change parameters or outputs).

Plugins SHOULD:
- Only orchestrate abra primitives (no raw pixel mutation the core can't already do).
- Encapsulate pipeline construction behind simple entry points.
- Avoid leaking internal temporary abstractions to public API.
- Provide clear examples and minimal configuration surface.

If plugin requires missing primitive → First propose adding that primitive to core using the template below, then build plugin atop it.

---
## 5. Process Workflow

1. Ideation → Run Decision Tree.
2. If Core: Draft Feature Proposal (see Template) → Review naming & placement → Implement → Add examples / tests → Benchmark (if performance sensitive).
3. If Plugin: Draft Plugin Proposal → Ensure no new primitives slip in → Implement orchestration → Provide usage samples.
4. Update docs: Add entry to appropriate section (`filters`, `transform`, `color`, etc.) or plugin README.
5. Track improvements or open questions in `docs/` (e.g. API review file).

---
## 6. Module & Naming Guidance

Core additions:
- Place files under the most specific existing domain folder (`filters/`, `color/`, `geometry/`, `combine/`, `draw/`, `adjustments/`).
- Public struct names: Descriptive, no abbreviations (e.g. `GaussianBlur`, `PerspectiveTransform`). Internal implementation helper: append `Inner` (e.g. `GaussianBlurInner`).
- Functions: Verb + noun when action oriented (`apply_blur`), noun when constructing (`Gradient`, `Mask`).
- Parameters: Prefer explicit ranges (e.g. `radius: u32`, `opacity: f32 (0.0–1.0)`).

Plugins:
- Keep a cohesive directory under `packages/plugins/<plugin-name>/`.
- Provide a concise entry type (`CollageLayout`, `PosterizePreset`) and builder or `apply_*` functions.
- Prefix strongly themed helpers internally if needed (`seasonal_*`). Avoid leaking theme words to core types.

---
## 7. Examples (Core vs Plugin)

| Idea | Analysis | Decision |
|------|----------|----------|
| New high-quality edge-aware upscaling | Novel sampling & pixel math | Core |
| Auto collage (grid + spacing + drop shadows) | Composition of layers, transforms, existing shadow effect | Plugin |
| L*a*b* color curve adjustment | New color space manipulation primitive | Core |
| Instagram-style vintage filter pack | Preset stack of existing adjustments & blends | Plugin |
| Fast Poisson blend | New blending algorithm | Core |
| Seasonal greeting card generator | Layout + themed assets | Plugin |

---
## 8. Anti-Patterns to Avoid

- Adding large preset pipelines directly to core.
- Introducing a primitive that duplicates an existing effect with slight parameter tweaks.
- Overloading a single API entry with unrelated concerns (e.g. geometry + color shift + blur in one function).
- Leaking experimental slow path into stable core without clear benchmark.
- Plugin performing raw pixel operations absent in core (indicates missing primitive).

---
## 9. Performance Considerations (for Core)

Include in proposal if applicable:
- Complexity class vs naive approach.
- Memory footprint (buffers, temp allocations, cache lines).
- Parallelization opportunities (thread-safe segmentation, tile processing).
- SIMD / GPU compatibility (existing abstractions in `gpu/` or potential future integration points).
- Avoiding redundant passes (combine operations when safe).
