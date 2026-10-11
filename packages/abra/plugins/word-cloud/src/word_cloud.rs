use abra::{
  abra_core::Resolution,
  canvas::prelude::*,
  typography::prelude::{FontLoader, Text},
};

use crate::WordCloudPlugin;

impl<'a> WordCloudPlugin<'a> {
  pub(crate) fn generate_cloud(&self) -> Canvas<'a> {
    let (width, height) = self.size.into();
    let resolution = self.options.as_ref().map(|opts| opts.resolution.unwrap()).unwrap_or(Resolution::SCREEN);

    let canvas = Canvas::new_blank("Word Cloud", width, height);
    canvas.set_resolution(resolution);

    let mut fonts = FontLoader::find_system_fonts();
    fonts.add_fonts(self.options.as_ref().and_then(|o| o.fonts.clone()).unwrap_or_default());
    let font_family = if self.options.as_ref().is_none() {
      fonts.load("Arial").unwrap()
    } else {
      fonts.load(&self.options.as_ref().and_then(|o| o.font_family.clone()).unwrap_or("Arial".to_string())).unwrap()
    };

    // Logic to generate the word cloud on the canvas goes here
    self.words.iter().for_each(|word| {
      let text_size = self.get_word_size(word);
      let text_appearance = self.options.as_ref().and_then(|o| o.text_appearance.clone()).unwrap_or_default();
      let text = Text::new(&font_family, &word.text).with_size(text_size).with_appearance(text_appearance);
      canvas.add_layer_from_image(&word.text, text, None);
    });

    let effects = self.options.clone().and_then(|opts| opts.effects).unwrap_or(LayerEffects::new());

    canvas.set_effects(effects.clone());

    canvas
  }
}
