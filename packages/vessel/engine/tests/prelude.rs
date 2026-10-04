//! An app can be written with only `use vessel_engine::prelude::*;`: no direct `pub-sub` import is needed for `subscribe`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use vessel_engine::prelude::*;

struct Fixed {
  changed: Arc<AtomicBool>,
}

impl MediaSource for Fixed {
  fn pacing(&self) -> Pacing {
    Pacing::OnDemand
  }

  fn has_changed(&self) -> bool {
    self.changed.load(Ordering::SeqCst)
  }

  fn render(&mut self, _delta: Duration) -> Frame {
    self.changed.store(false, Ordering::SeqCst);
    Frame {
      width: 1,
      height: 1,
      pixels: vec![1, 2, 3, 255],
    }
  }
}

#[test]
fn the_prelude_is_enough_to_run_a_view_and_listen_to_events() {
  let (sender, frames) = mpsc::channel();
  let engine = Engine::new(move |_id, frame: &Frame| {
    let _ = sender.send(frame.pixels.clone());
  });
  let changed = Arc::new(AtomicBool::new(true));
  let view = View::new(
    "test",
    Fixed {
      changed: changed.clone(),
    },
  );
  let _handler = view.subscribe(move |_: &u8| changed.store(true, Ordering::SeqCst));
  engine.add(&view);
  assert_eq!(frames.recv_timeout(Duration::from_secs(2)).unwrap(), vec![1, 2, 3, 255]);

  // The pub-sub traits come with the prelude: a Subject can be listened to and sent to.
  let subject = Subject::new();
  let heard = Arc::new(AtomicBool::new(false));
  let _subscription: Subscription = {
    let heard = heard.clone();
    subject.subscribe(move |_: &u8| heard.store(true, Ordering::SeqCst))
  };
  subject.next(1);
  assert!(heard.load(Ordering::SeqCst));
}
