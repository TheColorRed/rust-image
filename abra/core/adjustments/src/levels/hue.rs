use abra_core::{Image, ImageRef};
use options::{Apply, Options};

// TODO: Fix hue adjustment
/// Adjust the hue of an image where 0.0 is no change, -180.0 is -180 degrees, and 180.0 is 180 degrees.
#[derive(Clone)]
pub struct Hue {
  amount: i32,
  options: Options,
}

options::cpu_processor!(Hue);

impl Apply for Hue {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let _image = &mut image_ref as &mut Image;
    let _amount = self.amount;
    let _options = &self.options;
    // let amount = (amount as f32).clamp(-180.0, 180.0);
    // let amount = amount / 360.0; // Scale value to range [-0.5, 0.5] where 0.0 means no change

    // let mut colors = image.colors.view_mut();
    // colors.axis_iter_mut(Axis(1)).enumerate().for_each(|(i, mut color)| {
    //   println!("Original Color: {:?}, {:?}, {:?}", color[[0, 0]], color[[0, 1]], color[[0, 2]]);
    //   let (h, s, l) = rgb_to_hsl(color[[0, 0]], color[[0, 1]], color[[0, 2]]);
    //   let (r, g, b) = hsl_to_rgb(h + amount, s, l);
    //   color[[0, 0]] = r;
    //   color[[0, 1]] = g;
    //   color[[0, 2]] = b;
    //   println!("Updated Color: {:?}, {:?}, {:?}", color[[0, 0]], color[[0, 1]], color[[0, 2]]);
    // });

    // for i in 0..(image.color_len) as usize {
    //   let (h, s, l) = rgb_to_hsl(image.r[i], image.g[i], image.b[i]);
    //   let (r, g, b) = hsl_to_rgb(h + amount, s, l);
    //   image.r[i] = r;
    //   image.g[i] = g;
    //   image.b[i] = b;
    // }
  }
}

pub fn hue(p_amount: i32) -> Hue {
  Hue {
    amount: p_amount,
    options: None,
  }
}
