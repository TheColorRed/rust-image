# Usability: the app is for people who are not artists

The people who edit photos in Alakazam are not designers or photographers. They do not know that a darker color is "less
brightness", that "saturation" is how colorful something is, or that a hue bar is a color wheel laid flat. A control that is
correct but needs that knowledge is a control most people will not use. When adding or changing any edit control, work from
what a person would ask for ("make it warmer", "get rid of the shine", "make the background blurry"), not from what the library
calls it.

## Name things the way a person would say them

Use the plain word a person would use for what they see, not the word from the imaging trade.

| Library or trade word | Name in the app |
| --- | --- |
| Saturation | Colorfulness |
| Value / lightness (HSV) | Brightness |
| Opacity / blend amount | Strength |
| Feather / blur of a mask edge | Softness |
| Hue | Color |

- A label is one or two words, in the words people use ("Skin Smooth", "Red Eye", "Blemish").
- Name a group of controls after the thing it is for, not after how it works.
- Do not use a library name, a unit or a range in a label.
- The same setting has the same name everywhere. If "Strength" is how much of an effect shows in one control, it is not
  "Amount", "Intensity" or "Opacity" in the next.

## Offer the things people ask for by name

A slider reaches every value, but a person does not think in sliders. Put the common choices in front, as named swatches or
presets, and keep the sliders for fine-tuning.

- **Swatches** (`swatches` on a slider control with a `picker`): a row above the picker. Tapping one sets the sliders it lists
  and applies the control at once. Choose them from what people look for, so that a result needs no knowledge of which sliders
  to move to get it. The color picker of a control that tints something is the usual case: "black" should be one tap, not
  "brightness to zero".
- The swatch that matches the current values is shown as chosen, so the row also says what is applied.
- Give each swatch a label as well as its look: a dark swatch on a dark panel is hard to see, and the name is what people
  search for.
- Add a swatch for a thing people would ask for and could not otherwise find. Do not add one for every point on a slider.
- The same applies to a control that is only sliders: if there is a result people want that takes two or three sliders to
  reach, it is a preset.

## Show something at once

- Opening a control should apply a sensible starting look (`applyOnSelect` with a `defaultValue` that is a real effect), so
  the person has something to change and the control reads as applied. A control that opens on "nothing" and shows no change
  until a slider is moved looks broken.
- The preview follows the finger while a slider is dragged (`live` or `tool`), including a custom picker. A change the person
  has to let go to see is a change they cannot aim at.
- Order the tabs of a multi-slider control by what a person does first: what, then how much, then the finer points.
- A control that needs a model or something else that may be missing says so and offers to get it (`aiModelsRequested`). It
  does not open and silently do nothing.

## Defaults that look natural

- The default should be a natural, subtle use of the effect. Anything stronger is for the person to choose.
- Edges: a hard cut-out edge looks fake and a wide blur looks fuzzy. A control that works through a mask gets a softness slider,
  with a default that suits the thing it is applied to.
- Test an effect across the whole range a person can reach, not only the default and not only the easy cases. An effect that
  looks right at its default can look muddy, flat or blown out at the far ends of a slider or for other colors. Check a grid of
  the settings people will actually use, on real photos.
- Do that check offline, on the photos in `assets/`, and write the results to a contact sheet you can look at, instead of going
  back to the device after every change. `packages/ai/face-parsing/tests/lipstick_eval.rs` is an example: it runs the
  app's own maths over a grid of settings and writes one picture. Keep such a test in step with the code it mirrors.

## When there are several people

A control that applies to a person needs to know which one, but only when there is a choice.

- With one person in the photo, or none, do not show the "tap a person" bar or the outlines: the effect is for everyone, which
  is the same thing.
- With two or more, the person selector shows, and each control's values are kept for each person.
- A control that starts with a look when it opens (`applyOnSelect`) does that at once for one person or none. With two or more it
  puts nothing on until a person is tapped, and then only on that person. Never put it on everyone behind the person's back.
- The model that finds people may not be downloaded. Offer the download when a control needs it, and do not leave the control
  silently applying to the wrong thing.
