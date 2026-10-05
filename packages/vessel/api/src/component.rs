use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use pub_sub::{BehaviorSubject, Observable, Observer, Subject, Subscription};
use vessel_engine::{Frame, GpuFrame, MediaSource, Pacing, Waker};

use crate::layout::Edges;
use crate::layout::{Align, Container, Direction, Display, Hints, Justify, Length, Rect, Track, Units, layout};
use crate::theme::Background;
use crate::theme::ThemePatch;
use crate::{Canvas, Face, Font, GpuPicture, KeyEvent, PointerEvent, Quit, TextAlign, Theme};

/// A child, and the picture it last drew, kept until it changes.
struct Slot {
  component: Component,
  cache: Option<Frame>,
}

struct Inner {
  name: String,
  /// The size in pixels the component is drawn at. Whoever shows the component (a host, or a parent) sets it. A
  /// component with no size has nothing to draw.
  size: BehaviorSubject<(u32, u32)>,
  /// True when the component needs drawing again.
  dirty: AtomicBool,
  /// True when this component itself must be redrawn (not only one of its children).
  dirty_self: AtomicBool,
  /// True when at least one descendant changed (layout may or may not change).
  dirty_children: AtomicBool,
  /// Cached last composed frame, for partial (dirty-rect) updates.
  cached_frame: Mutex<Option<Frame>>,
  /// This component's drawing before its children and border, restored under child-only updates.
  /// Only parents need a separate layer; leaves already keep their final frame.
  cached_paint: Mutex<Option<Frame>>,
  /// Cached child layout (child identity + rect), to know when a partial update is safe.
  cached_placed: Mutex<Option<Vec<(usize, Rect)>>>,
  /// Wakes the engine, so a change is drawn even if the engine is asleep.
  waker: Mutex<Option<Waker>>,
  /// One `Subject` for each type of event the component has sent or been listened to for.
  events: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
  /// What the component watches. Dropped with it.
  watching: Mutex<Vec<Subscription>>,
  /// The picture the component shows instead of drawing one, when it was made on the GPU.
  gpu_frame: Mutex<Option<Arc<dyn GpuFrame>>>,
  /// The values the last `draw` read, each with the subscription that redraws the component when it changes.
  tracked: Mutex<HashMap<usize, Subscription>>,
  /// The streams it watches, held so a chain such as `a.map(..)` stays connected for as long as the component does.
  upstream: Mutex<Vec<Box<dyn Any + Send + Sync>>>,

  /// The text size and typeface this component sets for itself and everything inside it, or `None` to use its parent's.
  font_size: Mutex<Option<Units>>,
  font_face: Mutex<Option<Face>>,
  text_align: Mutex<Option<TextAlign>>,
  /// What a host says to use where nothing in the app has set a font. Read from the top of the tree.
  host_font: Mutex<Option<Font>>,
  /// False for a form control: it does not take the font from its parents, only from the host (like a browser's `<button>`).
  inherits_font: AtomicBool,
  /// The parts of the theme this component sets for itself and everything inside it.
  theme: Mutex<ThemePatch>,
  /// What a host says to use where nothing in the app has set a theme. Read from the top of the tree.
  host_theme: Mutex<Option<Theme>>,
  /// How this component places its children.
  container: Mutex<Container>,
  /// What this component asks of its parent's layout.
  hints: Mutex<Hints>,
  parent: Mutex<Weak<Inner>>,
  children: Mutex<Vec<Slot>>,
  /// The child that gets the keyboard.
  focused: Mutex<Option<Component>>,
  /// The child a pressed pointer button belongs to until it is released.
  captured: Mutex<Option<Component>>,
  /// Where the pointer last was, in this component's pixels.
  pointer: Mutex<(f32, f32)>,
  /// Whether this component already passes input on to its children.
  routing: AtomicBool,
}

impl Inner {
  fn wake(&self) {
    if let Some(waker) = self.waker.lock().unwrap().as_ref() {
      waker.wake();
    }
  }

  /// Notes that this component itself needs drawing again.
  fn changed_self(&self) {
    self.dirty.store(true, Ordering::SeqCst);
    self.dirty_self.store(true, Ordering::SeqCst);
    self.wake();
    if let Some(parent) = self.parent.lock().unwrap().upgrade() {
      parent.changed_children();
    }
  }

  /// Notes that a descendant changed, so the parent's picture needs updating.
  fn changed_children(&self) {
    self.dirty.store(true, Ordering::SeqCst);
    self.dirty_children.store(true, Ordering::SeqCst);
    self.wake();
    if let Some(parent) = self.parent.lock().unwrap().upgrade() {
      parent.changed_children();
    }
  }
}

/// A handle to a [`Component`] that does not keep it alive. A component's own listeners hold one of these to refer back to
/// it, because a listener that held the component itself would keep it alive forever.
#[derive(Clone)]
pub struct WeakComponent {
  inner: Weak<Inner>,
}

impl WeakComponent {
  /// The component, if it is still alive.
  pub fn upgrade(&self) -> Option<Component> {
    self.inner.upgrade().map(|inner| Component { inner })
  }
}

/// Anything the engine can render. Everything that is shown, added to a parent or mounted on a view must be one, so a
/// component that cannot be drawn does not compile.
///
/// A [`Component`] is, and so is a reusable component: a struct that keeps a `Component` inside and implements
/// `Deref<Target = Component>`, so its events, layout methods and `add` are available directly. The `Component` derive
/// writes that for an app's own type and checks that the field it names is renderable.
/// References and smart pointers to renderable types are renderable too, so a parent can add a borrowed control.
pub trait Renderable {
  /// The component that is drawn.
  fn component(&self) -> &Component;
}

impl Renderable for Component {
  fn component(&self) -> &Component {
    self
  }
}

impl<T: Deref> Renderable for T
where
  T::Target: Renderable,
{
  fn component(&self) -> &Component {
    self.deref().component()
  }
}

/// A thing on screen: it draws itself, reacts to changes, and sends and receives its own events.
///
/// - **Events** live on the component, and every one of them is a stream (a pub-sub `Subject`): input (`keyboard()`, `pointer()`), drawing
///   (`draw()`), and any event type with `subject::<MyEvent>()`. `component.next(MyEvent::Clicked)` sends one and
///   `component.subject::<MyEvent>().subscribe(|event| ...)` listens, for as long as the component lives.
/// - **`draw()`** is the stream you subscribe to in order to say how the component looks. Whatever properties a
///   listener reads, it depends on: when one changes the component redraws, with nothing declared.
/// - **Children**: `add` puts components inside it, placed as its [`Display`] says (stacked, flex or grid, like CSS). Any
///   component can have children. Each child is drawn at its own size, and input reaches it in its own coordinates.
/// - The **size** is set by whoever shows the component, and a component never knows where that is.
///
/// A component is a cheap handle: cloning gives another handle to the same component. It is also a [`MediaSource`], so
/// the engine can run it directly.
///
/// ```ignore
/// let level = BehaviorSubject::new(10u8);
/// let swatch = Component::new("swatch")
///   .subject::<Canvas>()
///   .subscribe({ let level = level.clone(); move |canvas| canvas.fill([level.value(), 0, 0, 255]) });
/// level.set(200);                        // redraws it: the listener read `level`
/// swatch.subject::<Clicked>().subscribe(|click| { /* the swatch was clicked */ });
///
/// let toolbar = Component::new("toolbar").with_display(Display::Flex).with_gap(8);
/// toolbar.add(swatch.width(120)).add(other.fill());
/// ```
#[derive(Clone)]
pub struct Component {
  inner: Arc<Inner>,
}

/// Makes `with_*` and `set_*` for one setting of how a component places its children. Changing it lays the children out
/// again and redraws.
macro_rules! setting {
  (into $with:ident, $set:ident, $field:ident, $type:ty, $about:literal) => {
    #[doc = concat!("Sets ", $about, ".")]
    pub fn $set(&self, p_value: impl Into<$type>) {
      self.inner.container.lock().unwrap().$field = p_value.into();
      self.inner.changed_self();
    }

    #[doc = concat!("Sets ", $about, ", and returns the component so calls can be chained.")]
    pub fn $with(&self, p_value: impl Into<$type>) -> Self {
      self.$set(p_value);
      self.clone()
    }
  };
  ($with:ident, $set:ident, $field:ident, $type:ty, $about:literal) => {
    #[doc = concat!("Sets ", $about, ".")]
    pub fn $set(&self, p_value: $type) {
      self.inner.container.lock().unwrap().$field = p_value;
      self.inner.changed_self();
    }

    #[doc = concat!("Sets ", $about, ", and returns the component so calls can be chained.")]
    pub fn $with(&self, p_value: $type) -> Self {
      self.$set(p_value);
      self.clone()
    }
  };
}

impl Component {
  /// Creates a component with no size and nothing to draw yet. Corner rounding follows the theme until explicitly set.
  /// Its children are stacked until
  /// [`with_display`](Self::with_display) says otherwise.
  /// - `p_name`: A name for telling components apart when debugging.
  pub fn new(p_name: impl Into<String>) -> Self {
    let component = Self {
      inner: Arc::new(Inner {
        name: p_name.into(),
        size: BehaviorSubject::new((0, 0)),
        dirty: AtomicBool::new(true),
        dirty_self: AtomicBool::new(true),
        dirty_children: AtomicBool::new(false),
        cached_frame: Mutex::new(None),
        cached_paint: Mutex::new(None),
        cached_placed: Mutex::new(None),
        waker: Mutex::new(None),
        events: Mutex::new(HashMap::new()),
        watching: Mutex::new(Vec::new()),
        gpu_frame: Mutex::new(None),
        tracked: Mutex::new(HashMap::new()),
        upstream: Mutex::new(Vec::new()),
        font_size: Mutex::new(None),
        font_face: Mutex::new(None),
        text_align: Mutex::new(None),
        host_font: Mutex::new(None),
        inherits_font: AtomicBool::new(true),
        theme: Mutex::new(ThemePatch::new()),
        host_theme: Mutex::new(None),
        container: Mutex::new(Container::default()),
        hints: Mutex::new(Hints::default()),
        parent: Mutex::new(Weak::new()),
        children: Mutex::new(Vec::new()),
        focused: Mutex::new(None),
        captured: Mutex::new(None),
        pointer: Mutex::new((0.0, 0.0)),
        routing: AtomicBool::new(false),
      }),
    };
    // A new size is a reason to draw again.
    component.watch(&component.inner.size);
    // A picture made on the GPU replaces what the component draws, until it is taken away.
    let weak = Arc::downgrade(&component.inner);
    component.events::<GpuPicture>().subscribe(move |picture| {
      if let Some(inner) = weak.upgrade() {
        *inner.gpu_frame.lock().unwrap() = picture.0.clone();
        inner.changed_self();
      }
    });
    component
  }

  /// The name the component was given.
  pub fn name(&self) -> &str {
    &self.inner.name
  }

  /// The size in pixels the component is drawn at, as a property: read it in `draw`, and whoever shows the component sets
  /// it.
  pub fn size(&self) -> &BehaviorSubject<(u32, u32)> {
    &self.inner.size
  }

  /// Sets the size, and returns the component so calls can be chained.
  pub fn with_size(&self, p_width: u32, p_height: u32) -> Self {
    self.inner.size.next((p_width, p_height));
    self.clone()
  }

  /// Redraws the component whenever `p_source` sends a value. Watch the properties that change how it looks. The
  /// component watches for as long as it lives. Returns the component so calls can be chained.
  pub fn watch<X: 'static>(&self, p_source: &impl Observable<X>) -> Self {
    let weak = Arc::downgrade(&self.inner);
    let subscription = p_source.subscribe(move |_| {
      if let Some(inner) = weak.upgrade() {
        inner.changed_self();
      }
    });
    self.inner.watching.lock().unwrap().push(subscription);
    self.inner.upstream.lock().unwrap().push(p_source.upstream());
    self.clone()
  }

  /// Whether `p_other` is a handle to this same component.
  pub fn is(&self, p_other: &Component) -> bool {
    Arc::ptr_eq(&self.inner, &p_other.inner)
  }

  /// A handle that does not keep the component alive.
  pub fn downgrade(&self) -> WeakComponent {
    WeakComponent {
      inner: Arc::downgrade(&self.inner),
    }
  }

  /// Ties `p_subscription` to the component's life: it is dropped, and so stops listening, when the component is. A
  /// reusable component uses this for its own listeners so its user does not have to hold them. Returns the component so
  /// calls can be chained.
  pub fn keep(&self, p_subscription: Subscription) -> Self {
    self.inner.watching.lock().unwrap().push(p_subscription);
    self.clone()
  }

  setting!(
    with_display,
    set_display,
    display,
    Display,
    "how the component places its children: stacked (the default), in a flex line, or in a grid"
  );
  setting!(
    with_direction,
    set_direction,
    direction,
    Direction,
    "which way a flex component lays its children: a row (the default) or a column"
  );
  setting!(with_wrap, set_wrap, wrap, bool, "whether a flex component moves children that do not fit to the next line");
  setting!(with_gap, set_gap, gap, u32, "the space in pixels between children");
  setting!(
    into
    with_padding,
    set_padding,
    padding,
    Edges,
    "the space inside the component, between its border and its children: a number for all sides, `[up_down, left_right]` or `[top, right, bottom, left]`, like CSS `padding`"
  );
  setting!(
    into
    with_margin,
    set_margin,
    margin,
    Edges,
    "the space around the component that its parent's layout keeps clear, in the same forms as `padding`, like CSS `margin`"
  );
  setting!(
    with_border_width,
    set_border_width,
    border_width,
    u32,
    "the width in pixels of the border drawn just inside the component's edge (0 for none), like CSS `border-width`"
  );
  /// Sets corner rounding in pixels or percent of the shorter border-box side. Plain integers mean pixels.
  /// `Units::Percent(50.0)` makes a pill or circle. Values are capped at half the shorter side.
  /// This setting is not inherited; call [`inherit_radius`](Self::inherit_radius) to follow the theme again.
  ///
  /// # Panics
  /// Panics if a percentage is negative or not finite.
  pub fn set_radius(&self, p_radius: impl Into<Units>) {
    let radius = p_radius.into();
    radius.validate();
    self.inner.container.lock().unwrap().radius = Some(radius);
    self.inner.changed_self();
  }

  /// Sets corner rounding and returns the component. See [`set_radius`](Self::set_radius).
  pub fn with_radius(&self, p_radius: impl Into<Units>) -> Self {
    self.set_radius(p_radius);
    self.clone()
  }

  /// Removes the explicit corner rounding so it follows the theme, as a new component does.
  pub fn inherit_radius(&self) {
    self.inner.container.lock().unwrap().radius = None;
    self.inner.changed_self();
  }
  setting!(
    with_background,
    set_background,
    background,
    Background,
    "what the component fills its area with before it draws (following the radius), like CSS `background`. Not inherited"
  );
  setting!(with_justify, set_justify, justify, Justify, "how spare room along the line is shared out");
  setting!(with_align, set_align, align, Align, "where children sit across the line they are in");
  /// Sets the widths of a grid component's columns, for example `[Track::Sized(Units::Pixels(200)), Track::Fr(1.0)]`.
  ///
  /// # Panics
  /// Panics if a sized track's percentage is negative or not finite.
  pub fn set_columns(&self, p_columns: impl IntoIterator<Item = Track>) {
    let columns: Vec<Track> = p_columns.into_iter().collect();
    for track in &columns {
      track.validate();
    }
    self.inner.container.lock().unwrap().columns = columns;
    self.inner.changed_self();
  }

  /// Sets the widths of a grid component's columns, and returns the component so calls can be chained.
  pub fn with_columns(&self, p_columns: impl IntoIterator<Item = Track>) -> Self {
    self.set_columns(p_columns);
    self.clone()
  }

  /// Sets the heights of a grid component's rows.
  ///
  /// # Panics
  /// Panics if a sized track's percentage is negative or not finite.
  pub fn set_rows(&self, p_rows: impl IntoIterator<Item = Track>) {
    let rows: Vec<Track> = p_rows.into_iter().collect();
    for track in &rows {
      track.validate();
    }
    self.inner.container.lock().unwrap().rows = rows;
    self.inner.changed_self();
  }

  /// Sets the heights of a grid component's rows, and returns the component so calls can be chained.
  pub fn with_rows(&self, p_rows: impl IntoIterator<Item = Track>) -> Self {
    self.set_rows(p_rows);
    self.clone()
  }

  /// The font the component draws text in. Its size and its typeface each come from the component if it set one, otherwise
  /// from the nearest parent that did, otherwise from the host (see [`set_default_font`](Self::set_default_font)), and
  /// failing that the built-in default. A canvas given to `draw` already has it, as [`Canvas::font`].
  pub fn font(&self) -> Font {
    let mut sizes = Vec::new();
    let mut face = None;
    let mut inner = Arc::clone(&self.inner);
    let mut reading = true;
    let host = loop {
      if reading {
        sizes.push(*inner.font_size.lock().unwrap());
        face = face.or_else(|| inner.font_face.lock().unwrap().clone());
        // A form control reads its own settings and then goes straight to the host.
        reading = inner.inherits_font.load(Ordering::SeqCst);
      }
      let parent = inner.parent.lock().unwrap().upgrade();
      match parent {
        Some(parent) => inner = parent,
        None => break inner.host_font.lock().unwrap().clone(),
      }
    };
    let base = host.unwrap_or_default();
    let size = sizes.into_iter().rev().fold(base.size, |inherited, units| {
      units.map_or(inherited, |units| units.resolve(inherited, inherited).round() as u32)
    });
    Font::new(face.unwrap_or(base.face), size)
  }

  /// Sets the text size in pixels, percent or em units relative to the inherited font size, and redraws its children.
  /// Plain integers mean pixels. Relative sizes are rounded to the nearest pixel and follow changes to the parent's font.
  /// A component that disables font inheritance uses the host's font size as its percentage basis.
  ///
  /// # Panics
  /// Panics if a percentage or em value is negative or not finite.
  pub fn set_font_size(&self, p_size: impl Into<Units>) {
    let size = p_size.into();
    size.validate();
    *self.inner.font_size.lock().unwrap() = Some(size);
    self.style_changed();
  }

  /// Sets the text size, and returns the component so calls can be chained.
  pub fn with_font_size(&self, p_size: impl Into<Units>) -> Self {
    self.set_font_size(p_size);
    self.clone()
  }

  /// The nearest explicit horizontal text alignment, or left alignment if none is set.
  /// Text alignment inherits independently of font inheritance.
  pub fn text_align(&self) -> TextAlign {
    let mut inner = Arc::clone(&self.inner);
    loop {
      if let Some(align) = *inner.text_align.lock().unwrap() {
        return align;
      }
      let parent = inner.parent.lock().unwrap().upgrade();
      match parent {
        Some(parent) => inner = parent,
        None => return TextAlign::default(),
      }
    }
  }

  /// Sets horizontal text alignment for this component and descendants without an override.
  pub fn set_text_align(&self, p_align: TextAlign) {
    *self.inner.text_align.lock().unwrap() = Some(p_align);
    self.style_changed();
  }

  /// Sets horizontal text alignment and returns the component.
  pub fn with_text_align(&self, p_align: TextAlign) -> Self {
    self.set_text_align(p_align);
    self.clone()
  }

  /// Restores horizontal text alignment inherited from the parent.
  pub fn inherit_text_align(&self) {
    *self.inner.text_align.lock().unwrap() = None;
    self.style_changed();
  }

  /// Sets the typeface for this component and everything inside it that has not set its own, and redraws them.
  pub fn set_font_face(&self, p_face: Face) {
    *self.inner.font_face.lock().unwrap() = Some(p_face);
    self.style_changed();
  }

  /// Sets the typeface, and returns the component so calls can be chained.
  pub fn with_font_face(&self, p_face: Face) -> Self {
    self.set_font_face(p_face);
    self.clone()
  }

  /// Sets whether the font is taken from the parents (the default, like CSS `font-family` and `font-size`). A form control
  /// turns it off, as a browser's `<button>` does: it uses its own settings and then the host's font, whatever its parents
  /// set. Turn it back on for a control to follow its parents, like CSS `font: inherit`.
  pub fn set_inherits_font(&self, p_inherits: bool) {
    self.inner.inherits_font.store(p_inherits, Ordering::SeqCst);
    self.style_changed();
  }

  /// Goes back to using the parent's size and typeface.
  pub fn inherit_font(&self) {
    *self.inner.font_size.lock().unwrap() = None;
    *self.inner.font_face.lock().unwrap() = None;
    self.style_changed();
  }

  /// For a host: the font to use where nothing in the app has set one. A desktop window gives its component the system's
  /// interface font, and a view inside another application gives the font of that application. Call it on the component
  /// the host shows.
  pub fn set_default_font(&self, p_font: Font) {
    *self.inner.host_font.lock().unwrap() = Some(p_font);
    self.style_changed();
  }

  /// The theme the component takes its look from: each value from the component if it set one, otherwise from the nearest
  /// parent that did, otherwise from the host (see [`set_default_theme`](Self::set_default_theme)), and failing that the
  /// light theme. A canvas given to `draw` already has it, as [`Canvas::theme`].
  pub fn theme(&self) -> Theme {
    let mut patch = ThemePatch::new();
    let mut inner = Arc::clone(&self.inner);
    let host = loop {
      patch = patch.or(&inner.theme.lock().unwrap());
      let parent = inner.parent.lock().unwrap().upgrade();
      match parent {
        Some(parent) => inner = parent,
        None => break inner.host_theme.lock().unwrap().clone(),
      }
    };
    patch.over(host.unwrap_or_default())
  }

  /// Changes some or all of the theme for this component and everything inside it, and redraws them. Pass a [`Theme`] to
  /// replace all of it, or a [`ThemePatch`] to change only the values it sets. This replaces any earlier patch set on this
  /// component.
  pub fn set_theme(&self, p_theme: impl Into<ThemePatch>) {
    *self.inner.theme.lock().unwrap() = p_theme.into();
    self.style_changed();
  }

  /// Changes the theme, and returns the component so calls can be chained.
  pub fn with_theme(&self, p_theme: impl Into<ThemePatch>) -> Self {
    self.set_theme(p_theme);
    self.clone()
  }

  /// Goes back to using the parent's theme.
  pub fn inherit_theme(&self) {
    *self.inner.theme.lock().unwrap() = ThemePatch::new();
    self.style_changed();
  }

  /// For a host: the theme to use where nothing in the app has set a value. A desktop window gives its component the
  /// user's system theme, and a view inside another application gives that application's. Call it on the component the
  /// host shows.
  pub fn set_default_theme(&self, p_theme: Theme) {
    *self.inner.host_theme.lock().unwrap() = Some(p_theme);
    self.style_changed();
  }

  /// Sets the color of the border (see [`set_border_width`](Self::set_border_width)). Without one the theme's border color
  /// is used.
  pub fn set_border_color(&self, p_rgba: [u8; 4]) {
    self.inner.container.lock().unwrap().border_color = Some(p_rgba);
    self.inner.changed_self();
  }

  /// Sets the color of the border, and returns the component so calls can be chained.
  pub fn with_border_color(&self, p_rgba: [u8; 4]) -> Self {
    self.set_border_color(p_rgba);
    self.clone()
  }

  /// Everything below this component may draw differently now, so all of it needs drawing again.
  fn style_changed(&self) {
    fn mark(p_inner: &Inner) {
      p_inner.dirty.store(true, Ordering::SeqCst);
      p_inner.dirty_self.store(true, Ordering::SeqCst);
      p_inner.dirty_children.store(true, Ordering::SeqCst);
      for slot in p_inner.children.lock().unwrap().iter() {
        mark(&slot.component.inner);
      }
    }
    mark(&self.inner);
    self.inner.changed_self();
  }

  /// Asks its parent's layout for a width in pixels or percent (see [`Units`]). Plain numbers mean pixels.
  ///
  /// # Panics
  /// Panics if a percentage is negative or not finite.
  pub fn set_width(&self, p_width: impl Into<Units>) {
    let width = Length::from(p_width.into());
    self.inner.hints.lock().unwrap().width = width;
    self.inner.changed_self();
  }

  /// Sets the width (see [`set_width`](Self::set_width)), and returns the component so calls can be chained.
  pub fn with_width(&self, p_width: impl Into<Units>) -> Self {
    self.set_width(p_width);
    self.clone()
  }

  /// Shorthand for [`with_width`](Self::with_width). Set it on the child, where it is used, not where it is defined.
  pub fn width(&self, p_width: impl Into<Units>) -> Self {
    self.with_width(p_width)
  }

  /// Asks its parent's layout for a height in pixels or percent (see [`Units`]). Plain numbers mean pixels.
  ///
  /// # Panics
  /// Panics if a percentage is negative or not finite.
  pub fn set_height(&self, p_height: impl Into<Units>) {
    let height = Length::from(p_height.into());
    self.inner.hints.lock().unwrap().height = height;
    self.inner.changed_self();
  }

  /// Sets the height (see [`set_height`](Self::set_height)), and returns the component so calls can be chained.
  pub fn with_height(&self, p_height: impl Into<Units>) -> Self {
    self.set_height(p_height);
    self.clone()
  }

  /// Shorthand for [`with_height`](Self::with_height).
  pub fn height(&self, p_height: impl Into<Units>) -> Self {
    self.with_height(p_height)
  }

  /// Asks its parent's layout for all the room that is left, in both directions, shared equally with the other children
  /// that fill. This is what a child does when it asks for nothing. Returns the component so calls can be chained.
  pub fn fill(&self) -> Self {
    let span = self.inner.hints.lock().unwrap().span;
    *self.inner.hints.lock().unwrap() = Hints {
      span,
      ..Hints::default()
    };
    self.inner.changed_self();
    self.clone()
  }

  /// Sets how big a share of the spare room along a flex line this child gets when it fills, like CSS `flex-grow`: a child
  /// with `grow(2.0)` gets twice as much as one with `grow(1.0)`. Returns the component so calls can be chained.
  pub fn grow(&self, p_share: f32) -> Self {
    self.inner.hints.lock().unwrap().grow = p_share;
    self.inner.changed_self();
    self.clone()
  }

  /// Makes this child cover `p_columns` grid columns and `p_rows` rows. Returns the component so calls can be chained.
  pub fn span(&self, p_columns: u16, p_rows: u16) -> Self {
    self.inner.hints.lock().unwrap().span = (p_columns, p_rows);
    self.inner.changed_self();
    self.clone()
  }

  /// Puts `p_child` inside this component, placed by its layout. The first child added has the keyboard until another is
  /// clicked or asks for it with [`focus`](Self::focus). A child belongs to one parent. Returns this component so calls
  /// can be chained.
  ///
  /// [`Quit`] is the one event that travels up on its own: a child's request to end the app reaches the parent, and so on
  /// to the top, where the host listens for it. Anything else a parent wants to hear from a child it subscribes to on the
  /// child.
  pub fn add(&self, p_child: impl Renderable) -> Self {
    let child = p_child.component().clone();
    *child.inner.parent.lock().unwrap() = Arc::downgrade(&self.inner);
    let first = {
      let mut children = self.inner.children.lock().unwrap();
      children.push(Slot {
        component: child.clone(),
        cache: None,
      });
      children.len() == 1
    };
    if first {
      *self.inner.focused.lock().unwrap() = Some(child.clone());
    }
    self.install_routing();

    let weak = Arc::downgrade(&self.inner);
    // Lives with the parent, which the listener refers to, so it uses the plain `Observable` subscription.
    let quit = child.events::<Quit>().subscribe(move |_: &Quit| {
      if let Some(inner) = weak.upgrade() {
        Component { inner }.next(Quit);
      }
    });
    self.inner.watching.lock().unwrap().push(quit);

    self.inner.changed_self();
    self.clone()
  }

  /// Gives this component the keyboard: its parent sends keys to it, and so on up the tree.
  pub fn focus(&self) {
    let parent = self.inner.parent.lock().unwrap().upgrade();
    if let Some(parent) = parent {
      *parent.focused.lock().unwrap() = Some(self.clone());
      Component { inner: parent }.focus();
    }
  }

  /// The pub-sub `Subject` for events of type `E` on this component: `subscribe` to listen, `next` to send, and use the
  /// operators on it to derive state without handlers (`button.subject::<Clicked>().scan(false, |on, _| !*on)` is a
  /// value that flips on every click). The ones a component has are:
  ///
  /// - [`KeyEvent`] and [`PointerEvent`]: the keys and the pointer while the component has them, in its own pixels. The host
  ///   sends them.
  /// - [`Canvas`]: each time the component needs drawing, every listener gets the same canvas of the component's size and
  ///   draws on it, in the order they subscribed. Children are drawn over it. A listener depends on whatever values it
  ///   reads, so the component redraws when one changes. Asking for it also asks for a redraw, so a listener added late
  ///   still gets its frame.
  /// - [`GpuPicture`]: send one to show a picture made on the GPU without copying it to the CPU.
  /// - [`Quit`]: a request to end the app. It travels up from a child to its parents, so the host listens at the top.
  ///
  /// A reusable component adds its own event types (`Clicked`, `Changed`) the same way.
  pub fn subject<E: Send + Sync + 'static>(&self) -> Subject<E> {
    if TypeId::of::<E>() == TypeId::of::<Canvas>() {
      self.inner.changed_self();
    }
    self.events::<E>()
  }

  /// The subject for events of type `E`, made the first time it is asked for.
  fn events<E: Send + Sync + 'static>(&self) -> Subject<E> {
    self
      .inner
      .events
      .lock()
      .unwrap()
      .entry(TypeId::of::<E>())
      .or_insert_with(|| Box::new(Subject::<E>::persistent()))
      .downcast_ref::<Subject<E>>()
      .expect("the subject stored for an event type is a Subject of that type")
      .clone()
  }

  /// Each child with the rectangle its parent's layout gives it, in drawing order (the last is in front).
  fn placed_children(&self) -> Vec<(Component, Rect)> {
    let children = self.inner.children.lock().unwrap();
    let hints: Vec<Hints> = children
      .iter()
      .map(|slot| Hints {
        font_size: slot.component.font().size,
        margin: slot.component.inner.container.lock().unwrap().margin,
        ..*slot.component.inner.hints.lock().unwrap()
      })
      .collect();
    let mut style = self.inner.container.lock().unwrap().clone();
    style.font_size = self.font().size;
    let rects = layout(&style, self.inner.size.value(), &hints);
    children.iter().map(|slot| slot.component.clone()).zip(rects).collect()
  }

  /// Places the children and gives each one its rectangle as its size. Laying out is needed to draw and also to route
  /// input, which can arrive before the first draw, so both do it.
  fn layout_children(&self) -> Vec<(Component, Rect)> {
    let placed = self.placed_children();
    for (child, rect) in &placed {
      // Only a real change: setting the size wakes the child, and layout runs on every draw.
      let size = (rect.width, rect.height);
      if child.inner.size.peek() != size {
        child.inner.size.next(size);
      }
    }
    placed
  }

  /// Starts passing the keyboard and the pointer on to the children. Done once, when the first child is added.
  fn install_routing(&self) {
    if self.inner.routing.swap(true, Ordering::SeqCst) {
      return;
    }
    let weak = Arc::downgrade(&self.inner);
    let keys = self.events::<KeyEvent>().subscribe({
      let weak = weak.clone();
      move |event| {
        if let Some(inner) = weak.upgrade() {
          Component { inner }.route_key(event);
        }
      }
    });
    let pointer = self.events::<PointerEvent>().subscribe(move |event| {
      if let Some(inner) = weak.upgrade() {
        Component { inner }.route_pointer(event);
      }
    });
    let mut watching = self.inner.watching.lock().unwrap();
    watching.push(keys);
    watching.push(pointer);
  }

  /// Keys go to the child that has the keyboard.
  fn route_key(&self, p_event: &KeyEvent) {
    let focused = self.inner.focused.lock().unwrap().clone();
    if let Some(child) = focused {
      child.next(p_event.clone());
    }
  }

  /// Pointer moves go to the child under the pointer, or to the child holding a pressed button, in that child's pixels. A
  /// press also gives that child the keyboard.
  fn route_pointer(&self, p_event: &PointerEvent) {
    let placed = self.layout_children();
    let hit = |x: f32, y: f32| placed.iter().rev().find(|(_, rect)| rect.contains(x, y)).cloned();
    let placed_at = |child: &Component| {
      placed.iter().find(|(other, _)| Arc::ptr_eq(&other.inner, &child.inner)).map(|(_, rect)| (child.clone(), *rect))
    };

    match p_event {
      PointerEvent::Moved { x, y } => {
        *self.inner.pointer.lock().unwrap() = (*x, *y);
        let captured = self.inner.captured.lock().unwrap().clone();
        let target = match captured {
          Some(child) => placed_at(&child),
          None => hit(*x, *y),
        };
        if let Some((child, rect)) = target {
          child.next(PointerEvent::Moved {
            x: x - rect.x as f32,
            y: y - rect.y as f32,
          });
        }
      }
      PointerEvent::Button { button, pressed } => {
        let (x, y) = *self.inner.pointer.lock().unwrap();
        let target = if *pressed {
          let target = hit(x, y);
          *self.inner.captured.lock().unwrap() = target.as_ref().map(|(child, _)| child.clone());
          if let Some((child, _)) = &target {
            *self.inner.focused.lock().unwrap() = Some(child.clone());
          }
          target
        } else {
          let captured = self.inner.captured.lock().unwrap().take();
          match captured {
            Some(child) => placed_at(&child),
            None => hit(x, y),
          }
        };
        if let Some((child, _)) = target {
          child.next(PointerEvent::Button {
            button: *button,
            pressed: *pressed,
          });
        }
      }
    }
  }

  /// Runs `p_draw`, and makes the component redraw whenever a property (or other value) it read changes. This is why a
  /// `draw` needs no declaration of what it depends on: it depends on whatever it reads.
  fn draw_tracking_reads(&self, p_draw: impl FnOnce()) {
    let seen = Arc::new(Mutex::new(HashSet::new()));
    let weak = Arc::downgrade(&self.inner);
    pub_sub::tracking::track(
      {
        let seen = Arc::clone(&seen);
        move |read| {
          seen.lock().unwrap().insert(read.id);
          let Some(inner) = weak.upgrade() else { return };
          if inner.tracked.lock().unwrap().contains_key(&read.id) {
            return;
          }
          let again = weak.clone();
          let subscription = (read.on_change)(Arc::new(move || {
            if let Some(inner) = again.upgrade() {
              inner.changed_self();
            }
          }));
          inner.tracked.lock().unwrap().insert(read.id, subscription);
        }
      },
      p_draw,
    );
    // What it did not read this time it no longer depends on.
    self.inner.tracked.lock().unwrap().retain(|id, _| seen.lock().unwrap().contains(id));
  }

  /// Draws the component and everything inside it. Child-only updates restore this component's cached drawing
  /// under the dirty region, without running its drawing listeners or changing their reactive subscriptions.
  fn compose(&self) -> Frame {
    fn intersects(p_rect: &Rect, p_clip: (i32, i32, i32, i32)) -> bool {
      let (left, top, right, bottom) = p_clip;
      let r_left = p_rect.x;
      let r_top = p_rect.y;
      let r_right = p_rect.x + p_rect.width as i32;
      let r_bottom = p_rect.y + p_rect.height as i32;
      r_left < right && r_right > left && r_top < bottom && r_bottom > top
    }

    let (width, height) = self.inner.size.value();

    // Every child is given its rectangle first. A new size marks things as changed, which is why the flag is cleared only
    // afterwards: laying out is not a reason to draw again.
    let placed = self.layout_children();

    // Cleared before drawing, so a change that happens while drawing asks for another frame.
    let dirty_self = self.inner.dirty_self.swap(false, Ordering::SeqCst);
    let _dirty_children = self.inner.dirty_children.swap(false, Ordering::SeqCst);
    self.inner.dirty.store(false, Ordering::SeqCst);

    let placed_ids: Vec<(usize, Rect)> =
      placed.iter().map(|(child, rect)| (Arc::as_ptr(&child.inner) as usize, *rect)).collect();

    let layout_unchanged = {
      let cached = self.inner.cached_placed.lock().unwrap();
      cached.as_ref().is_some_and(|old| *old == placed_ids)
    };

    let theme = self.theme();
    let style = self.inner.container.lock().unwrap().clone();
    let border = style.border_width;
    let border_color = style.border_color.unwrap_or(theme.border);

    // Compose children (only the ones that changed) and compute the dirty union in parent coordinates.
    let (dirty_union, child_frames): (Option<(i32, i32, i32, i32)>, Vec<Option<Frame>>) = {
      let mut children = self.inner.children.lock().unwrap();
      let mut dirty_union: Option<(i32, i32, i32, i32)> = None; // left, top, right, bottom
      let mut frames = Vec::with_capacity(children.len());
      for (slot, (_, rect)) in children.iter_mut().zip(&placed) {
        let child_dirty = slot.cache.is_none() || slot.component.inner.dirty.load(Ordering::SeqCst);
        if child_dirty {
          slot.cache = Some(slot.component.compose());
          let left = rect.x;
          let top = rect.y;
          let right = rect.x + rect.width as i32;
          let bottom = rect.y + rect.height as i32;
          dirty_union = Some(match dirty_union {
            None => (left, top, right, bottom),
            Some((l, t, r, b)) => (l.min(left), t.min(top), r.max(right), b.max(bottom)),
          });
        }
        frames.push(slot.cache.clone());
      }
      (dirty_union, frames)
    };

    // If we can safely reuse the existing pixels, only touch the dirty area.
    let paint = self.inner.cached_paint.lock().unwrap().clone();
    let can_partial = !dirty_self
      && layout_unchanged
      && (dirty_union.is_none() || paint.as_ref().is_some_and(|frame| (frame.width, frame.height) == (width, height)));

    if can_partial {
      let cached = self.inner.cached_frame.lock().unwrap().take();
      if let Some(cached) = cached {
        if cached.width == width && cached.height == height {
          // No child actually re-rendered: return the cached frame.
          if dirty_union.is_none() {
            *self.inner.cached_frame.lock().unwrap() = Some(cached.clone());
            *self.inner.cached_placed.lock().unwrap() = Some(placed_ids);
            return cached;
          }

          let (mut left, mut top, mut right, mut bottom) = dirty_union.unwrap();
          // Expand for AA safety.
          left -= 1;
          top -= 1;
          right += 1;
          bottom += 1;
          left = left.clamp(0, width as i32);
          top = top.clamp(0, height as i32);
          right = right.clamp(0, width as i32);
          bottom = bottom.clamp(0, height as i32);

          if left < right && top < bottom {
            let clip_w = (right - left) as u32;
            let clip_h = (bottom - top) as u32;

            let mut pixels_vec = match Arc::try_unwrap(cached.pixels) {
              Ok(vec) => vec,
              Err(shared) => (*shared).clone(),
            };

            // Replace the dirty region with the parent's prepared drawing, including transparent pixels.
            // Blending the old background here would accumulate alpha or leave retired foreground pixels behind.
            let paint = paint.as_ref().unwrap();
            for row in top as usize..bottom as usize {
              let start = (row * width as usize + left as usize) * 4;
              let end = (row * width as usize + right as usize) * 4;
              pixels_vec[start..end].copy_from_slice(&paint.pixels[start..end]);
            }

            let mut canvas = Canvas::with_pixels(width, height, pixels_vec);
            canvas.set_font(self.font());
            canvas.set_text_align(self.text_align());
            canvas.set_clip(Some((left, top, clip_w, clip_h)));

            let shorter = width.min(height);
            let radius = style
              .radius
              .map_or(theme.radius as f64, |units| units.resolve(shorter, canvas.font().size))
              .clamp(0.0, shorter as f64 / 2.0) as f32;

            {
              let mut pixels = canvas.pixels.lock().unwrap();
              if canvas.intact(&pixels) {
                let clip = (left, top, right, bottom);
                for (frame, (_, rect)) in child_frames.iter().zip(&placed) {
                  if !intersects(rect, clip) {
                    continue;
                  }
                  if let Some(frame) = frame {
                    canvas.draw_pixels_into(
                      pixels.as_mut_slice(),
                      rect.x,
                      rect.y,
                      frame.width,
                      frame.height,
                      frame.pixels.as_slice(),
                    );
                  }
                }
              }
            }

            if border > 0 {
              canvas.stroke_rounded_rect(0, 0, width, height, radius, border as f32, border_color);
            }

            let pixels = Arc::new(canvas.into_pixels());
            let frame = Frame {
              width,
              height,
              pixels: Arc::clone(&pixels),
            };
            *self.inner.cached_frame.lock().unwrap() = Some(frame.clone());
            *self.inner.cached_placed.lock().unwrap() = Some(placed_ids);
            return frame;
          }

          // Dirty union collapsed to empty after clamping; return cached frame.
          *self.inner.cached_frame.lock().unwrap() = Some(cached.clone());
          *self.inner.cached_placed.lock().unwrap() = Some(placed_ids);
          return cached;
        }
      }
    }

    // Full redraw path.
    let mut canvas = Canvas::new(width, height);
    canvas.set_font(self.font());
    canvas.set_text_align(self.text_align());
    let shorter = width.min(height);
    let radius = style
      .radius
      .map_or(theme.radius as f64, |units| units.resolve(shorter, canvas.font().size))
      .clamp(0.0, shorter as f64 / 2.0) as f32;
    if let Some(color) = style.background.resolve(&theme) {
      canvas.fill_rounded_rect(0, 0, width, height, radius, color);
    }
    let content = (
      (style.padding.left + border) as i32,
      (style.padding.top + border) as i32,
      width.saturating_sub(style.padding.horizontal() + border * 2),
      height.saturating_sub(style.padding.vertical() + border * 2),
    );
    canvas.set_box(radius, content);
    canvas.set_theme(theme);
    self.draw_tracking_reads(|| self.events::<Canvas>().next(canvas.clone()));

    // Retain the prepared background/content separately from foreground children. Reactive subscriptions from
    // this draw remain attached while child-only updates reuse it; any of those properties invalidates this layer.
    *self.inner.cached_paint.lock().unwrap() = if child_frames.is_empty() {
      None
    } else {
      Some(Frame {
        width,
        height,
        pixels: Arc::new(canvas.pixels()),
      })
    };

    {
      let mut pixels = canvas.pixels.lock().unwrap();
      if canvas.intact(&pixels) {
        for (frame, (_, rect)) in child_frames.iter().zip(&placed) {
          if let Some(frame) = frame {
            canvas.draw_pixels_into(
              pixels.as_mut_slice(),
              rect.x,
              rect.y,
              frame.width,
              frame.height,
              frame.pixels.as_slice(),
            );
          }
        }
      }
    }

    if border > 0 {
      canvas.stroke_rounded_rect(0, 0, width, height, radius, border as f32, border_color);
    }

    let pixels = Arc::new(canvas.into_pixels());
    let frame = Frame {
      width,
      height,
      pixels: Arc::clone(&pixels),
    };
    *self.inner.cached_frame.lock().unwrap() = Some(frame.clone());
    *self.inner.cached_placed.lock().unwrap() = Some(placed_ids);
    frame
  }
}

impl<E: Send + Sync + 'static> Observer<E> for Component {
  fn next(&self, p_event: E) {
    self.events::<E>().next(p_event);
  }
}

impl MediaSource for Component {
  fn pacing(&self) -> Pacing {
    Pacing::OnDemand
  }

  fn has_changed(&self) -> bool {
    let (width, height) = self.inner.size.value();
    width > 0 && height > 0 && self.inner.dirty.load(Ordering::SeqCst)
  }

  fn render(&mut self, _p_delta: Duration) -> Frame {
    self.compose()
  }

  fn render_gpu(&mut self, _p_delta: Duration) -> Option<Arc<dyn GpuFrame>> {
    let frame = self.inner.gpu_frame.lock().unwrap().clone();
    if frame.is_some() {
      self.inner.dirty.store(false, Ordering::SeqCst);
    }
    frame
  }

  fn set_waker(&mut self, p_waker: Waker) {
    *self.inner.waker.lock().unwrap() = Some(p_waker);
  }
}

#[cfg(test)]
mod tests {
  use std::sync::mpsc;

  use vessel_engine::{Engine, View};

  use super::*;
  use crate::PointerButton;

  const WAIT: Duration = Duration::from_secs(2);
  const QUIET: Duration = Duration::from_millis(150);

  #[derive(Clone, Debug, PartialEq)]
  enum Clicked {
    Once,
  }

  #[derive(Clone, Debug, PartialEq)]
  struct Moved(i32);

  /// Runs `p_component` on an engine and returns each frame it draws.
  fn run_frames(p_component: &Component) -> (Engine, mpsc::Receiver<Frame>) {
    let (sender, frames) = mpsc::channel();
    let engine = Engine::new(move |_id, frame: &Frame| {
      let _ = sender.send(frame.clone());
    });
    engine.add(&View::<()>::new(p_component.name(), p_component.clone()));
    (engine, frames)
  }

  /// Runs `p_component` and returns what it draws: the first byte of each frame, and its width.
  fn run(p_component: &Component) -> (Engine, mpsc::Receiver<(u8, u32)>) {
    let (sender, frames) = mpsc::channel();
    let engine = Engine::new(move |_id, frame: &Frame| {
      let _ = sender.send((frame.pixels[0], frame.width));
    });
    engine.add(&View::<()>::new(p_component.name(), p_component.clone()));
    (engine, frames)
  }

  /// The RGBA pixel at (`p_x`, `p_y`) of a frame.
  fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_frame.width + p_x) * 4) as usize;
    p_frame.pixels[at..at + 4].try_into().unwrap()
  }

  /// A component that lays its children out in a flex line.
  fn flex(p_direction: Direction) -> Component {
    Component::new("flex").with_display(Display::Flex).with_direction(p_direction)
  }

  /// A component that draws with `p_draw`: the one place the tests build a drawing component in a single expression.
  fn drawing(p_component: Component, p_draw: impl Fn(&Canvas) + Send + Sync + 'static) -> Component {
    p_component.subject::<Canvas>().subscribe(p_draw);
    p_component
  }

  /// A component that fills itself with `p_rgba`.
  fn swatch(p_name: &str, p_rgba: [u8; 4]) -> Component {
    drawing(Component::new(p_name), move |canvas| canvas.fill(p_rgba))
  }

  /// A recorder of what a stream sends.
  fn record<T: Clone + Send + Sync + 'static>(p_stream: &Subject<T>) -> (Arc<Mutex<Vec<T>>>, Subscription) {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let subscription = {
      let heard = Arc::clone(&heard);
      p_stream.subscribe(move |event| heard.lock().unwrap().push(event.clone()))
    };
    (heard, subscription)
  }

  fn key(p_name: &str, p_pressed: bool) -> KeyEvent {
    KeyEvent {
      key: p_name.to_string(),
      pressed: p_pressed,
    }
  }

  fn moved(p_x: f32, p_y: f32) -> PointerEvent {
    PointerEvent::Moved { x: p_x, y: p_y }
  }

  fn button(p_pressed: bool) -> PointerEvent {
    PointerEvent::Button {
      button: PointerButton::Left,
      pressed: p_pressed,
    }
  }

  // --- events, drawing and watching

  #[test]
  fn events_are_typed_and_each_type_has_its_own_stream() {
    let component = Component::new("test");
    let clicks = Arc::new(Mutex::new(Vec::new()));
    let moves = Arc::new(Mutex::new(Vec::new()));
    let _clicks = {
      let clicks = Arc::clone(&clicks);
      component.subject::<Clicked>().subscribe(move |event| clicks.lock().unwrap().push(event.clone()))
    };
    let _moves = {
      let moves = Arc::clone(&moves);
      component.subject::<Moved>().subscribe(move |event| moves.lock().unwrap().push(event.clone()))
    };

    component.next(Clicked::Once);
    component.next(Moved(3));
    component.next(Moved(4));
    assert_eq!(*clicks.lock().unwrap(), vec![Clicked::Once]);
    assert_eq!(*moves.lock().unwrap(), vec![Moved(3), Moved(4)]);
  }

  #[test]
  fn clones_are_the_same_component_and_the_listener_lives_with_the_component() {
    let component = Component::new("test");
    let count = Arc::new(Mutex::new(0));
    component.subject::<Clicked>().subscribe({
      let count = Arc::clone(&count);
      move |_| *count.lock().unwrap() += 1
    });
    component.clone().next(Clicked::Once);
    component.next(Clicked::Once);
    assert_eq!(*count.lock().unwrap(), 2, "a clone is the same component, and nobody holds the listener");

    drop(component);
    assert_eq!(Arc::strong_count(&count), 1, "the listener was dropped with the component");
  }

  #[test]
  fn a_subscription_from_a_component_stream_can_stop_one_listener_early() {
    let component = Component::new("test");
    let count = Arc::new(Mutex::new(0));
    let subscription = {
      let count = Arc::clone(&count);
      component.subject::<KeyEvent>().subscribe(move |_| *count.lock().unwrap() += 1)
    };
    component.next(key("a", true));
    subscription.unsubscribe();
    component.next(key("b", true));
    assert_eq!(*count.lock().unwrap(), 1);
  }

  #[test]
  fn a_component_with_no_size_has_nothing_to_draw() {
    let component = drawing(Component::new("test"), |canvas| canvas.fill([1, 1, 1, 255]));
    assert!(!component.clone().has_changed());
    component.size().next((4, 4));
    assert!(component.clone().has_changed());
  }

  #[test]
  fn the_engine_draws_a_component_and_redraws_it_when_a_watched_property_changes() {
    let level = BehaviorSubject::new(10u8);
    let component = drawing(Component::new("swatch").with_size(2, 1), {
      let level = level.clone();
      move |canvas| canvas.fill([level.value(), 0, 0, 255])
    })
    .watch(&level);
    let (_engine, frames) = run(&component);

    assert_eq!(frames.recv_timeout(WAIT), Ok((10, 2)));
    level.next(20);
    assert_eq!(frames.recv_timeout(WAIT), Ok((20, 2)));
  }

  #[test]
  fn a_property_read_in_draw_redraws_without_being_watched_and_one_that_is_not_read_does_not() {
    let (read, unread) = (BehaviorSubject::new(10u8), BehaviorSubject::new(0u8));
    let component = drawing(Component::new("swatch").with_size(1, 1), {
      let read = read.clone();
      move |canvas| canvas.fill([read.value(), 0, 0, 255])
    });
    let (_engine, frames) = run(&component);

    assert_eq!(frames.recv_timeout(WAIT), Ok((10, 1)));
    read.next(20);
    assert_eq!(frames.recv_timeout(WAIT), Ok((20, 1)), "draw read it, so it depends on it");
    unread.next(5);
    assert!(frames.recv_timeout(QUIET).is_err(), "nothing read this one");
  }

  #[test]
  fn what_draw_stops_reading_is_forgotten_and_reads_inside_nested_calls_count() {
    let (switch, first, second) = (BehaviorSubject::new(true), BehaviorSubject::new(1u8), BehaviorSubject::new(2u8));
    let component = drawing(Component::new("swatch").with_size(1, 1), {
      let (switch, first, second) = (switch.clone(), first.clone(), second.clone());
      move |canvas| {
        // The reads happen inside another closure, as with `fill_with`.
        let pick = || if switch.value() { first.value() } else { second.value() };
        canvas.fill_with(|_, _| [pick() as f32 / 255.0, 0.0, 0.0]);
      }
    });
    let (_engine, frames) = run(&component);
    assert_eq!(frames.recv_timeout(WAIT), Ok((1, 1)));

    second.next(7); // not read while the switch is on
    assert!(frames.recv_timeout(QUIET).is_err());
    switch.next(false);
    assert_eq!(frames.recv_timeout(WAIT), Ok((7, 1)));
    first.next(9); // no longer read
    assert!(frames.recv_timeout(QUIET).is_err());
    second.next(8);
    assert_eq!(frames.recv_timeout(WAIT), Ok((8, 1)));
  }

  #[test]
  fn a_new_size_redraws_the_component_at_that_size() {
    let component = drawing(Component::new("swatch").with_size(2, 1), |canvas| canvas.fill([5, 0, 0, 255]));
    let (_engine, frames) = run(&component);

    assert_eq!(frames.recv_timeout(WAIT), Ok((5, 2)));
    component.size().next((6, 3));
    assert_eq!(frames.recv_timeout(WAIT), Ok((5, 6)));
  }

  #[test]
  fn watching_a_chain_of_operators_keeps_the_chain_alive() {
    let level = BehaviorSubject::new(1u8);
    // The middle of the chain is never stored in a variable.
    let component = drawing(Component::new("swatch").with_size(1, 1), {
      let level = level.clone();
      move |canvas| canvas.fill([level.value(), 0, 0, 255])
    })
    .watch(&level.map(|value: &u8| *value));
    let (_engine, frames) = run(&component);

    assert_eq!(frames.recv_timeout(WAIT), Ok((1, 1)));
    level.next(2);
    assert_eq!(frames.recv_timeout(WAIT), Ok((2, 1)));
  }

  #[test]
  fn dropping_a_component_releases_what_it_drew_with_and_watched() {
    let marker = Arc::new(());
    let level = BehaviorSubject::new(1u8);
    let component = {
      let marker = Arc::clone(&marker);
      drawing(Component::new("swatch"), move |_| {
        let _keep_alive = &marker;
      })
      .watch(&level)
    };
    assert_eq!(Arc::strong_count(&marker), 2);
    drop(component);
    assert_eq!(Arc::strong_count(&marker), 1);
  }

  // --- input

  #[test]
  fn input_from_a_host_arrives_as_the_neutral_streams() {
    let component = Component::new("test");
    let (keys, _keys) = record(&component.subject::<KeyEvent>());
    let (moves, _moves) = record(&component.subject::<PointerEvent>());

    // This is all a host does: it sends events to the component.
    component.next(key("a", true));
    component.next(moved(1.0, 2.0));
    assert_eq!(*keys.lock().unwrap(), vec![key("a", true)]);
    assert_eq!(*moves.lock().unwrap(), vec![moved(1.0, 2.0)]);

    // `subscribe` on the component hears the same events as the accessor.
    let heard = Arc::new(Mutex::new(0));
    let _direct = {
      let heard = Arc::clone(&heard);
      component.subject::<KeyEvent>().subscribe(move |_| *heard.lock().unwrap() += 1)
    };
    component.next(key("b", true));
    assert_eq!(*heard.lock().unwrap(), 1);
  }

  #[test]
  fn keys_become_a_running_state_that_redraws_the_component() {
    let swatch = Component::new("swatch").with_size(1, 1);
    // Key presses -> steps -> a running level, all with operators.
    let level = swatch
      .subject::<KeyEvent>()
      .filter_map(|event| match (event.pressed, event.key.as_str()) {
        (true, "ArrowUp") => Some(10i32),
        (true, "ArrowDown") => Some(-10),
        _ => None,
      })
      .scan(100i32, |level, step| (level + step).clamp(0, 255));
    swatch.subject::<Canvas>().subscribe({
      let level = level.clone();
      move |canvas| canvas.fill([level.value() as u8, 0, 0, 255])
    });
    let (_engine, frames) = run(&swatch);

    assert_eq!(frames.recv_timeout(WAIT), Ok((100, 1)));
    swatch.next(key("ArrowUp", true));
    assert_eq!(frames.recv_timeout(WAIT), Ok((110, 1)));
    swatch.next(key("ArrowUp", false)); // releasing is not a step
    swatch.next(key("x", true)); // other keys are not steps
    assert!(frames.recv_timeout(QUIET).is_err());
    swatch.next(key("ArrowDown", true));
    assert_eq!(frames.recv_timeout(WAIT), Ok((100, 1)));
  }

  // --- children and layout

  #[test]
  fn a_row_draws_its_children_side_by_side() {
    let row = flex(Direction::Row).with_size(4, 2);
    row.add(swatch("red", [255, 0, 0, 255]).width(1)).add(swatch("blue", [0, 0, 255, 255]));
    let (_engine, frames) = run_frames(&row);

    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!((frame.width, frame.height), (4, 2));
    assert_eq!(pixel(&frame, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 0, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 1, 0), [0, 0, 255, 255]);
    assert_eq!(pixel(&frame, 3, 1), [0, 0, 255, 255]);
  }

  #[test]
  fn children_are_given_their_size_and_follow_the_container() {
    let (left, right) = (swatch("left", [1, 0, 0, 255]), swatch("right", [2, 0, 0, 255]));
    let row = flex(Direction::Row).with_size(10, 4);
    row.add(&left).add(&right);
    let (_engine, frames) = run_frames(&row);
    frames.recv_timeout(WAIT).unwrap();
    assert_eq!((left.size().value(), right.size().value()), ((5, 4), (5, 4)));

    row.size().next((20, 6));
    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!((frame.width, frame.height), (20, 6));
    assert_eq!((left.size().value(), right.size().value()), ((10, 6), (10, 6)));
    assert_eq!(pixel(&frame, 9, 0), [1, 0, 0, 255]);
    assert_eq!(pixel(&frame, 10, 0), [2, 0, 0, 255]);
  }

  #[test]
  fn a_container_redraws_when_a_child_changes_and_only_that_child_is_drawn_again() {
    let level = BehaviorSubject::new(10u8);
    let draws_of_other = Arc::new(Mutex::new(0));
    let changing = drawing(Component::new("changing"), {
      let level = level.clone();
      move |canvas| canvas.fill([level.value(), 0, 0, 255])
    })
    .watch(&level);
    let other = drawing(Component::new("other"), {
      let draws = Arc::clone(&draws_of_other);
      move |canvas| {
        *draws.lock().unwrap() += 1;
        canvas.fill([0, 9, 0, 255]);
      }
    });
    let row = flex(Direction::Row).with_size(2, 1);
    row.add(&changing).add(&other);
    let (_engine, frames) = run_frames(&row);

    assert_eq!(pixel(&frames.recv_timeout(WAIT).unwrap(), 0, 0)[0], 10);
    level.next(30);
    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!(pixel(&frame, 0, 0)[0], 30);
    assert_eq!(pixel(&frame, 1, 0), [0, 9, 0, 255]);
    assert_eq!(*draws_of_other.lock().unwrap(), 1, "the unchanged child kept its picture");
  }

  #[test]
  fn containers_nest_and_children_draw_over_the_parents_own_drawing() {
    let column = drawing(flex(Direction::Column).with_size(4, 4), |canvas| canvas.fill([50, 50, 50, 255]));
    let top = flex(Direction::Row).height(2);
    top.add(swatch("a", [255, 0, 0, 255])).add(swatch("b", [0, 255, 0, 255]));
    column.add(top).add(Component::new("clear")); // the second child draws nothing and stays transparent
    let (_engine, frames) = run_frames(&column);

    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!(pixel(&frame, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 3, 1), [0, 255, 0, 255]);
    assert_eq!(pixel(&frame, 0, 2), [50, 50, 50, 255], "the transparent child shows the parent's drawing");
  }

  #[test]
  fn a_stack_draws_later_children_in_front() {
    let stack = Component::new("stack").with_size(2, 1);
    stack.add(swatch("back", [10, 0, 0, 255])).add(swatch("front", [0, 20, 0, 255]).width(1));
    let (_engine, frames) = run_frames(&stack);

    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!(pixel(&frame, 0, 0), [0, 20, 0, 255]);
    assert_eq!(pixel(&frame, 1, 0), [10, 0, 0, 255]);
  }

  #[test]
  fn size_and_typeface_are_each_inherited_from_the_nearest_parent_that_sets_one() {
    let (grandchild, child) = (Component::new("grandchild"), Component::new("child"));
    let page = Component::new("page");
    child.add(&grandchild);
    page.add(&child);
    assert_eq!(grandchild.font(), Font::default(), "nobody set one");

    page.set_font_size(24);
    assert_eq!((child.font().size, grandchild.font().size), (24, 24));

    child.set_font_size(8);
    assert_eq!((page.font().size, child.font().size, grandchild.font().size), (24, 8, 8));

    child.inherit_font();
    assert_eq!(grandchild.font().size, 24);
  }

  #[test]
  fn em_font_sizes_compose_with_percentages_and_host_inheritance() {
    let root = Component::new("root").with_font_size(Units::Em(1.5));
    root.set_default_font(Font::new(Face::bitmap(), 20));
    let child = Component::new("child").with_font_size(Units::Em(2.0));
    let leaf = Component::new("leaf").with_font_size(Units::Percent(50.0));
    root.add(&child);
    child.add(&leaf);
    assert_eq!((root.font().size, child.font().size, leaf.font().size), (30, 60, 30));
    root.set_default_font(Font::new(Face::bitmap(), 10));
    assert_eq!((root.font().size, child.font().size, leaf.font().size), (15, 30, 15));
    child.set_inherits_font(false);
    assert_eq!((child.font().size, leaf.font().size), (20, 10));
    child.inherit_font();
    assert_eq!((child.font().size, leaf.font().size), (10, 5));
  }

  #[test]
  fn em_dimensions_follow_the_child_font_in_stack_flex_and_grid() {
    for display in [Display::Stack, Display::Flex, Display::Grid] {
      let parent = Component::new("parent").with_size(200, 100).with_display(display).with_font_size(10);
      let child =
        Component::new("child").with_font_size(Units::Em(2.0)).with_width(Units::Em(3.0)).with_height(Units::Em(1.5));
      parent.add(&child);
      parent.clone().render(Duration::ZERO);
      assert_eq!(child.size().value(), (60, 30), "{display:?}");
      parent.set_font_size(20);
      parent.clone().render(Duration::ZERO);
      assert_eq!(child.size().value(), (120, 60), "{display:?}");
      parent.set_font_size(50);
      parent.clone().render(Duration::ZERO);
      assert_eq!(child.size().value(), (200, 100), "em sizes still shrink to fit: {display:?}");
    }
  }

  #[test]
  fn em_grid_tracks_use_the_grid_font_not_the_child_font() {
    let grid = Component::new("grid")
      .with_display(Display::Grid)
      .with_size(200, 100)
      .with_font_size(10)
      .with_columns([Track::Sized(Units::Em(4.0)), Track::Fr(1.0)])
      .with_rows([Track::Sized(Units::Em(2.0)), Track::Fr(1.0)]);
    let first = Component::new("first").with_font_size(50);
    let second = Component::new("second");
    grid.add(&first).add(&second);
    grid.clone().render(Duration::ZERO);
    assert_eq!(first.size().value(), (40, 20));
    assert_eq!(second.size().value(), (160, 20));
    grid.set_font_size(20);
    grid.clone().render(Duration::ZERO);
    assert_eq!(first.size().value(), (80, 40));
    assert_eq!(second.size().value(), (120, 40));
  }

  #[test]
  fn em_radii_preserve_fractional_pixels_follow_fonts_and_clamp() {
    let seen = Arc::new(Mutex::new(0.0));
    let component = Component::new("rounded").with_size(100, 60).with_font_size(13).with_radius(Units::Em(0.5));
    component.subject::<Canvas>().subscribe({
      let seen = Arc::clone(&seen);
      move |canvas| *seen.lock().unwrap() = canvas.radius()
    });
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 6.5);
    component.set_font_size(20);
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 10.0);
    component.set_radius(Units::Em(10.0));
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 30.0);
  }

  #[test]
  fn invalid_em_values_leave_existing_settings_usable() {
    let component = Component::new("em")
      .with_size(100, 80)
      .with_display(Display::Grid)
      .with_font_size(12)
      .with_radius(3)
      .with_columns([Track::Sized(Units::Em(2.0))])
      .with_rows([Track::Sized(Units::Em(1.0))]);
    let child = Component::new("child").with_width(Units::Em(1.0)).with_height(Units::Em(0.5));
    component.add(&child);
    for value in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
      let em = Units::Em(value);
      let setters: [fn(&Component, Units); 4] = [
        |component, units| component.set_font_size(units),
        |component, units| component.set_radius(units),
        |component, units| component.set_width(units),
        |component, units| component.set_height(units),
      ];
      for setter in setters {
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| setter(&child, em))).is_err());
      }
      assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| component.set_columns([Track::Sized(em)]))).is_err()
      );
      assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| component.set_rows([Track::Sized(em)]))).is_err()
      );
    }
    component.clone().render(Duration::ZERO);
    assert_eq!(child.font().size, 12);
    assert_eq!(child.size().value(), (12, 6));
    child.set_width(Units::Em(0.0));
    component.clone().render(Duration::ZERO);
    assert_eq!(child.size().value(), (0, 6));
  }

  #[test]
  fn percentage_font_sizes_resolve_each_ancestor_once_and_follow_changes() {
    let page = Component::new("page").with_font_size(Units::Pixels(20));
    let child = Component::new("child").with_font_size(Units::Percent(150.0));
    let grandchild = Component::new("grandchild").with_font_size(Units::Percent(50.0));
    let inherited = Component::new("inherited");
    page.add(&child);
    child.add(&grandchild);
    grandchild.add(&inherited);
    assert_eq!((child.font().size, grandchild.font().size, inherited.font().size), (30, 15, 15));
    page.set_font_size(30);
    assert_eq!((child.font().size, grandchild.font().size, inherited.font().size), (45, 23, 23));
    child.inherit_font();
    assert_eq!((child.font().size, grandchild.font().size), (30, 15));
    grandchild.set_font_size(Units::Pixels(9));
    assert_eq!(inherited.font().size, 9);
  }

  #[test]
  fn percentage_font_sizes_use_the_host_at_roots_and_inheritance_boundaries() {
    let page = Component::new("page").with_font_size(Units::Percent(200.0));
    page.set_default_font(Font::new(Face::bitmap(), 13));
    let control = Component::new("control").with_font_size(Units::Percent(150.0));
    let child = Component::new("child").with_font_size(Units::Percent(50.0));
    control.set_inherits_font(false);
    page.add(&control);
    control.add(&child);
    assert_eq!((page.font().size, control.font().size, child.font().size), (26, 20, 10));
    page.set_default_font(Font::new(Face::bitmap(), 20));
    assert_eq!((page.font().size, control.font().size, child.font().size), (40, 30, 15));
    control.set_inherits_font(true);
    assert_eq!((control.font().size, child.font().size), (60, 30));
    page.inherit_font();
    assert_eq!((control.font().size, child.font().size), (30, 15));
  }

  #[test]
  fn invalid_font_percentages_leave_the_previous_font_usable() {
    let component = Component::new("font").with_font_size(Units::Pixels(12));
    for percent in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
      assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
          component.set_font_size(Units::Percent(percent));
        }))
        .is_err()
      );
      assert_eq!(component.font().size, 12);
    }
    component.set_font_size(Units::Percent(0.0));
    assert_eq!(component.font().size, 0);
  }

  #[test]
  fn a_component_that_does_not_inherit_the_font_uses_its_own_settings_then_the_hosts() {
    let (control, page) = (Component::new("control"), Component::new("page"));
    page.add(&control);
    page.set_default_font(Font::new(Face::bitmap(), 13));
    page.set_font_size(40);
    control.set_inherits_font(false);
    assert_eq!(control.font().size, 13, "the page's 40 is ignored, the host's 13 is used");

    control.set_font_size(20);
    assert_eq!(control.font().size, 20, "its own setting still wins");
    control.set_inherits_font(true);
    control.inherit_font();
    assert_eq!(control.font().size, 40);
  }

  #[test]
  fn the_host_supplies_the_font_where_the_app_sets_none_and_the_app_can_override_part_of_it() {
    let system = Face::system();
    let (child, page) = (Component::new("child"), Component::new("page"));
    page.add(&child);
    page.set_default_font(Font::new(system.clone(), 13));
    assert_eq!(child.font(), Font::new(system.clone(), 13), "it reaches the children");

    // Setting only a size keeps the host's typeface, and the other way round.
    child.set_font_size(30);
    assert_eq!(child.font(), Font::new(system, 30));
    child.inherit_font();
    child.set_font_face(Face::bitmap());
    assert_eq!(child.font(), Font::new(Face::bitmap(), 13));
  }

  #[test]
  fn a_canvas_is_given_the_font_the_component_ended_up_with_and_a_change_above_redraws_below() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let child = drawing(Component::new("child"), {
      let seen = Arc::clone(&seen);
      move |canvas| seen.lock().unwrap().push(canvas.font().size)
    });
    let page = Component::new("page").with_size(4, 4);
    page.add(&child);
    let mut source = page.clone();

    source.render(Duration::ZERO);
    page.set_font_size(32);
    assert!(source.has_changed(), "the page is drawn again");
    source.render(Duration::ZERO);
    page.inherit_font();
    source.render(Duration::ZERO);
    page.set_default_font(Font::new(Face::bitmap(), 40));
    source.render(Duration::ZERO);
    assert_eq!(
      *seen.lock().unwrap(),
      vec![16, 32, 16, 40],
      "the child was redrawn each time with the font it inherited"
    );
  }

  #[test]
  fn every_component_has_a_box_with_background_border_padding_and_rounded_corners() {
    let child = swatch("child", [0, 255, 0, 255]);
    let boxed = Component::new("boxed")
      .with_size(20, 20)
      .with_radius(0)
      .with_background(Background::Color([255, 0, 0, 255]))
      .with_border_width(2)
      .with_border_color([0, 0, 255, 255])
      .with_padding(3);
    boxed.add(&child);
    let (_engine, frames) = run_frames(&boxed);

    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!(pixel(&frame, 0, 0), [0, 0, 255, 255], "the border is the outer 2 pixels");
    assert_eq!(pixel(&frame, 2, 2), [255, 0, 0, 255], "then the padding shows the background");
    assert_eq!(pixel(&frame, 5, 5), [0, 255, 0, 255], "then the child, inside border and padding");
    assert_eq!(child.size().value(), (10, 10));

    // Rounded corners cut the background and the border away at the corners.
    boxed.set_radius(8);
    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!(pixel(&frame, 0, 0)[3], 0);
    assert_eq!(pixel(&frame, 10, 0), [0, 0, 255, 255], "but the straight part of the border is still there");
  }

  #[test]
  fn a_canvas_knows_where_the_content_goes_and_how_round_the_corners_are() {
    let seen = Arc::new(Mutex::new(None));
    let component = Component::new("c").with_size(30, 20).with_padding([2, 4]).with_border_width(1).with_radius(5);
    component.subject::<Canvas>().subscribe({
      let seen = Arc::clone(&seen);
      move |canvas| *seen.lock().unwrap() = Some((canvas.content_rect(), canvas.radius()))
    });
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), Some(((5, 3, 20, 14), 5.0)));
  }

  #[test]
  fn invalid_grid_tracks_leave_existing_settings_usable() {
    let component = Component::new("grid")
      .with_display(Display::Grid)
      .with_size(100, 80)
      .with_columns([Track::Sized(Units::Percent(25.0)), Track::Fr(1.0)])
      .with_rows([Track::Sized(Units::Percent(50.0))]);
    let child = Component::new("child");
    component.add(&child);
    for percent in [-1.0, f32::NAN, f32::INFINITY] {
      let tracks = [Track::Sized(Units::Percent(percent))];
      assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| component.set_columns(tracks))).is_err());
      assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| component.set_rows(tracks))).is_err());
    }
    component.clone().render(Duration::ZERO);
    assert_eq!(child.size().value(), (25, 40));
  }

  #[test]
  fn unit_radii_follow_size_and_can_return_to_the_theme_default() {
    let seen = Arc::new(Mutex::new(0.0));
    let component = Component::new("rounded")
      .with_size(80, 40)
      .with_padding(5)
      .with_border_width(2)
      .with_radius(Units::Percent(25.0));
    component.subject::<Canvas>().subscribe({
      let seen = Arc::clone(&seen);
      move |canvas| *seen.lock().unwrap() = canvas.radius()
    });
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 10.0, "the basis is the shorter border-box side, not the content");
    component.size().next((20, 60));
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 5.0);
    component.set_radius(Units::Percent(200.0));
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 10.0, "radius is capped at half the shorter side");
    component.set_radius(Units::Pixels(3));
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 3.0);
    component.set_theme(ThemePatch::new().radius(7.0));
    component.inherit_radius();
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 7.0);
    component.set_theme(ThemePatch::new().radius(2.0));
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 2.0);
    component.size().next((0, 60));
    component.clone().render(Duration::ZERO);
    assert_eq!(*seen.lock().unwrap(), 0.0);
  }

  #[test]
  fn percentage_radii_retain_fractional_resolution_and_reject_invalid_values() {
    let component = Component::new("rounded").with_size(7, 9).with_radius(Units::Percent(25.0));
    component.subject::<Canvas>().subscribe(|canvas| assert_eq!(canvas.radius(), 1.75));
    component.clone().render(Duration::ZERO);
    for percent in [-1.0, f32::NAN, f32::INFINITY] {
      assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
          component.set_radius(Units::Percent(percent));
        }))
        .is_err()
      );
    }
    component.clone().render(Duration::ZERO);
  }

  #[test]
  fn a_margin_keeps_a_child_clear_of_its_neighbours_and_the_edges() {
    let (left, right) = (swatch("left", [1, 0, 0, 255]), swatch("right", [2, 0, 0, 255]));
    let row = flex(Direction::Row).with_size(20, 10);
    row.add(left.with_margin(2)).add(&right);
    row.clone().render(Duration::ZERO);
    assert_eq!(
      (left.size().value(), right.size().value()),
      ((8, 6), (8, 10)),
      "the 16 pixels left after left's margins are shared; left is 4 shorter for its margins"
    );
    assert_eq!(
      (
        row.clone().render(Duration::ZERO).pixels[(2 * 20 + 2) * 4],
        row.clone().render(Duration::ZERO).pixels[(2 * 20 + 1) * 4]
      ),
      (1, 0),
      "the margin itself is empty"
    );
  }

  struct Picture;

  impl GpuFrame for Picture {
    fn size(&self) -> (u32, u32) {
      (2, 2)
    }

    fn as_any(&self) -> &dyn std::any::Any {
      self
    }
  }

  #[test]
  fn a_gpu_picture_replaces_what_the_component_draws_until_it_is_taken_away() {
    let component = drawing(Component::new("swatch").with_size(2, 2), |canvas| canvas.fill([5, 0, 0, 255]));
    let mut source = component.clone();
    assert!(source.render_gpu(Duration::ZERO).is_none(), "nothing from the GPU yet");

    component.subject::<GpuPicture>().next(GpuPicture(Some(Arc::new(Picture))));
    assert!(source.has_changed(), "a new picture is a reason to draw");
    assert!(source.render_gpu(Duration::ZERO).is_some());
    assert!(!source.has_changed(), "and drawing it is done");
    assert!(source.render_gpu(Duration::ZERO).is_some(), "it stays, so a resize shows it again");

    component.subject::<GpuPicture>().next(GpuPicture(None));
    assert!(source.render_gpu(Duration::ZERO).is_none(), "back to drawing on the CPU");
    assert!(source.has_changed());
  }

  #[test]
  fn any_component_can_hold_children_and_how_it_places_them_is_a_setting() {
    let (left, right) = (swatch("left", [1, 0, 0, 255]), swatch("right", [2, 0, 0, 255]));
    let parent = Component::new("parent").with_size(10, 4);
    parent.add(&left.width(3)).add(&right);
    let (_engine, frames) = run_frames(&parent);

    // Stacked by default: both start at the top left, the later one in front.
    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!((pixel(&frame, 0, 0), pixel(&frame, 9, 3)), ([2, 0, 0, 255], [2, 0, 0, 255]));

    // Changing the setting lays the same children out again and redraws.
    parent.set_display(Display::Flex);
    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!((pixel(&frame, 2, 0), pixel(&frame, 3, 0)), ([1, 0, 0, 255], [2, 0, 0, 255]));

    // Another setting does too.
    parent.set_gap(2);
    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!(pixel(&frame, 3, 0), [0, 0, 0, 0], "the gap between the children is empty");
    assert_eq!(pixel(&frame, 5, 0), [2, 0, 0, 255]);
  }

  #[test]
  fn a_grid_component_places_children_in_cells_and_a_child_can_span() {
    let grid = Component::new("grid")
      .with_size(40, 20)
      .with_display(Display::Grid)
      .with_columns([Track::Sized(Units::Pixels(10)), Track::Fr(1.0)])
      .with_rows([Track::Fr(1.0), Track::Fr(1.0)]);
    let (a, b, c) = (swatch("a", [1, 0, 0, 255]), swatch("b", [2, 0, 0, 255]), swatch("c", [3, 0, 0, 255]));
    grid.add(&a).add(&b).add(c.span(2, 1));
    let (_engine, frames) = run_frames(&grid);

    let frame = frames.recv_timeout(WAIT).unwrap();
    assert_eq!(pixel(&frame, 0, 0), [1, 0, 0, 255]);
    assert_eq!(pixel(&frame, 39, 0), [2, 0, 0, 255]);
    assert_eq!(pixel(&frame, 0, 19), [3, 0, 0, 255], "the spanning child covers both columns");
    assert_eq!(pixel(&frame, 39, 19), [3, 0, 0, 255]);
  }

  #[test]
  fn a_reusable_component_can_be_added_because_it_is_renderable() {
    struct Badge {
      component: Component,
    }
    impl Deref for Badge {
      type Target = Component;

      fn deref(&self) -> &Component {
        &self.component
      }
    }

    let row = flex(Direction::Row).with_size(2, 1);
    row.add(Badge {
      component: swatch("badge", [7, 0, 0, 255]),
    });
    let (_engine, frames) = run_frames(&row);
    assert_eq!(pixel(&frames.recv_timeout(WAIT).unwrap(), 0, 0), [7, 0, 0, 255]);
  }

  // --- routing input to children

  #[test]
  fn pointer_moves_go_to_the_child_under_the_pointer_in_its_own_pixels() {
    let (left, right) = (Component::new("left"), Component::new("right"));
    let row = flex(Direction::Row).with_size(100, 10);
    row.add(&left).add(&right);
    let (left_moves, _l) = record(&left.subject::<PointerEvent>());
    let (right_moves, _r) = record(&right.subject::<PointerEvent>());

    row.next(moved(75.0, 5.0));
    row.next(moved(10.0, 2.0));
    assert_eq!(*right_moves.lock().unwrap(), vec![moved(25.0, 5.0)]);
    assert_eq!(*left_moves.lock().unwrap(), vec![moved(10.0, 2.0)]);
  }

  #[test]
  fn a_pressed_button_stays_with_its_child_until_it_is_released() {
    let (left, right) = (Component::new("left"), Component::new("right"));
    let row = flex(Direction::Row).with_size(100, 10);
    row.add(&left).add(&right);
    let (left_events, _l) = record(&left.subject::<PointerEvent>());
    let (right_events, _r) = record(&right.subject::<PointerEvent>());

    row.next(moved(10.0, 5.0));
    row.next(button(true)); // pressed over the left child
    row.next(moved(75.0, 5.0)); // dragged over the right one
    row.next(button(false)); // released there

    assert_eq!(*left_events.lock().unwrap(), vec![moved(10.0, 5.0), button(true), moved(75.0, 5.0), button(false)]);
    assert!(right_events.lock().unwrap().is_empty(), "the drag belongs to the child that was pressed");
  }

  #[test]
  fn keys_go_to_the_first_child_until_a_click_or_focus_moves_them() {
    let (first, second) = (Component::new("first"), Component::new("second"));
    let row = flex(Direction::Row).with_size(100, 10);
    row.add(&first).add(&second);
    let (first_keys, _f) = record(&first.subject::<KeyEvent>());
    let (second_keys, _s) = record(&second.subject::<KeyEvent>());

    row.next(key("a", true)); // the first child added has the keyboard
    row.next(moved(75.0, 5.0));
    row.next(button(true)); // clicking the second gives it the keyboard
    row.next(button(false));
    row.next(key("b", true));
    first.focus(); // and a child can ask for it
    row.next(key("c", true));

    assert_eq!(*first_keys.lock().unwrap(), vec![key("a", true), key("c", true)]);
    assert_eq!(*second_keys.lock().unwrap(), vec![key("b", true)]);
  }

  #[test]
  fn input_is_routed_down_through_nested_containers_and_coordinates_add_up() {
    let target = Component::new("target");
    let inner = flex(Direction::Row);
    inner.add(Component::new("spacer").width(30)).add(&target);
    let outer = flex(Direction::Column).with_size(100, 100);
    outer.add(Component::new("header").height(20)).add(&inner);
    let (moves, _m) = record(&target.subject::<PointerEvent>());
    let (keys, _k) = record(&target.subject::<KeyEvent>());

    // The target is 30 across and 20 down inside the outer column.
    outer.next(moved(40.0, 25.0));
    assert_eq!(*moves.lock().unwrap(), vec![moved(10.0, 5.0)]);

    // A click on it gives it the keyboard all the way up.
    outer.next(button(true));
    outer.next(button(false));
    outer.next(key("x", true));
    assert_eq!(*keys.lock().unwrap(), vec![key("x", true)]);
  }

  #[test]
  fn quit_travels_up_from_a_child_to_the_top() {
    let child = Component::new("child");
    let middle = flex(Direction::Row);
    middle.add(&child);
    let top = flex(Direction::Column);
    top.add(&middle);
    let quits = Arc::new(Mutex::new(0));
    let _quits = {
      let quits = Arc::clone(&quits);
      top.subject::<Quit>().subscribe(move |_| *quits.lock().unwrap() += 1)
    };

    child.next(Quit);
    assert_eq!(*quits.lock().unwrap(), 1);
  }

  #[test]
  fn other_events_do_not_travel_up() {
    let child = Component::new("child");
    let parent = flex(Direction::Row);
    parent.add(&child);
    let (heard, _subscription) = {
      let heard = Arc::new(Mutex::new(0));
      let subscription = {
        let heard = Arc::clone(&heard);
        parent.subject::<Clicked>().subscribe(move |_| *heard.lock().unwrap() += 1)
      };
      (heard, subscription)
    };

    child.next(Clicked::Once);
    assert_eq!(*heard.lock().unwrap(), 0, "a parent hears a child only by subscribing on the child");
  }

  #[test]
  fn a_listener_on_a_component_stream_lasts_as_long_as_the_component_with_nothing_held() {
    let heard = Arc::new(Mutex::new(0));
    let component = Component::new("c");
    component.subject::<Clicked>().subscribe({
      let heard = Arc::clone(&heard);
      move |_| *heard.lock().unwrap() += 1
    });
    component.next(Clicked::Once);
    component.clone().next(Clicked::Once);
    assert_eq!(*heard.lock().unwrap(), 2);

    let weak = component.downgrade();
    drop(component);
    assert!(weak.upgrade().is_none(), "the handler does not keep the component alive");
    assert_eq!(Arc::strong_count(&heard), 1, "and it was freed with it");
  }

  #[test]
  fn a_kept_subscription_lives_as_long_as_the_component_and_no_longer() {
    let marker = Arc::new(());
    let component = Component::new("kept");
    let weak = component.downgrade();
    component.keep(component.subject::<PointerEvent>().subscribe({
      let marker = Arc::clone(&marker);
      let weak = weak.clone();
      move |_| {
        let _keep_alive = &marker;
        // It refers back to the component through a weak handle, so it does not keep the component alive.
        let _ = weak.upgrade();
      }
    }));
    assert_eq!(Arc::strong_count(&marker), 2);
    assert!(weak.upgrade().is_some());

    drop(component);
    assert!(weak.upgrade().is_none(), "the component was released although its listener refers to it");
    assert_eq!(Arc::strong_count(&marker), 1, "and the listener went with it");
  }
}
