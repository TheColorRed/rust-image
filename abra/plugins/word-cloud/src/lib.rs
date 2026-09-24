use core::ops::Range;

use abra::{
  abra_core::{IntoNumber, Resolution, Size},
  canvas::prelude::LayerEffects,
  prelude::*,
  typography::prelude::{Font, Text, TextAppearance, TextSize, TextStyle},
};

mod word_cloud;

pub struct Word {
  /// The weight of the word. The higher the weight, the more prominent the word will be in the cloud.
  pub(crate) weight: f32,
  /// The text of the word to be displayed in the cloud.
  pub(crate) text: String,
}

impl Word {
  pub fn new(p_weight: impl IntoNumber, p_text: String) -> Self {
    Word {
      weight: p_weight.into(),
      text: p_text,
    }
  }
}

pub struct WordCloudPlugin<'a> {
  words: Vec<Word>,
  size: Size,
  options: Option<WordCloudOptions<'a>>,
}

#[derive(Clone)]
pub struct WordCloudOptions<'a> {
  area: Area,
  effects: Option<LayerEffects<'a>>,
  weights: Range<f32>,
  resolution: Option<Resolution>,
  fonts: Option<Vec<String>>,
  font_family: Option<String>,
  text_appearance: Option<TextAppearance>,
}

impl<'a> WordCloudPlugin<'a> {
  pub fn new<I: Into<WordCloudOptions<'a>>>(p_size: Size) -> Self {
    WordCloudPlugin {
      words: Vec::new(),
      size: p_size,
      options: None,
    }
  }

  pub fn with_options(mut self, p_options: WordCloudOptions<'a>) -> Self {
    self.options = Some(p_options);
    self
  }

  /// Gets the smallest number and the largest number among the word weights.
  ///
  /// Returns a tuple `(min, max)`. If there are no words, returns
  /// `(f32::INFINITY, f32::NEG_INFINITY)`.
  fn range(&self) -> (f32, f32) {
    self.words.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(smallest, largest), word| {
      (smallest.min(word.weight), largest.max(word.weight))
    })
  }

  fn get_word_size(&self, p_word: &Word) -> TextSize {
    let (smallest_weight, largest_weight) = self.range();
    let smallest_weight = if smallest_weight.is_infinite() { 14.0 } else { smallest_weight };
    let largest_weight = if largest_weight.is_infinite() { 24.0 } else { largest_weight };

    // Scale the word's weight to a text size between the smallest and largest weights.
    let weight_ratio = if largest_weight == smallest_weight {
      1.0
    } else {
      (p_word.weight - smallest_weight) / (largest_weight - smallest_weight)
    };

    TextSize::points(smallest_weight + weight_ratio * (largest_weight - smallest_weight))
  }
}

impl<'a> Plugin<'a> for WordCloudPlugin<'a> {
  fn name(&self) -> &str {
    "WordCloud"
  }

  fn description(&self) -> &str {
    "Generates a word cloud from the input text."
  }

  fn apply(&mut self) -> Result<PluginResult<'a>, PluginError> {
    let result = PluginResult::new();
    self.generate_cloud();
    Ok(result)
  }
}
