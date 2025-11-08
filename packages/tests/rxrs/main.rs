#![allow(unused_imports)]
use std::sync::RwLock;

use rxrs::{
  operators::map,
  subject::{BehaviorSubject, ReplaySubject, Subject},
  subscription::Subscription,
  traits::{Observer, Subscribable},
};

struct Person {
  name: BehaviorSubject<String>,
  initials: RwLock<String>,
}

impl AsRef<Person> for Person {
  fn as_ref(&self) -> &Person {
    &self
  }
}

impl Person {
  pub fn new(name: &str) -> Self {
    let name = BehaviorSubject::new(name.to_string());
    let item = Self {
      name,
      initials: RwLock::new("".to_string()),
    };

    item.name.subscribe(|v| println!("Name: {v}"));
    // {
    //   let item_ref = item.as_ref();
    //   item.name.subscribe(move |v| {
    //     let initials = v.split(' ').map(|s| s.chars().next().unwrap()).collect::<String>();
    //     item_ref.initials = RwLock::new(initials);
    //   });
    // }
    item
  }

  pub fn set_name(&self, name: &str) {
    self.name.next(name.to_string());
  }
}

fn main() {
  let person = Person::new("Alice Smith");
  person.set_name("Bob Johnson");

  // println!("Name: {}", person.name());

  // let subject = ReplaySubject::new(3);
  // let s = subject.subscribe(|v| println!("Received 1: {v}"));

  // subject.next(100);
  // subject.next(200);
  // subject.next(300);
  // subject.next(400);
  // subject.next(500);
  // subject.next(600);

  // println!("---");

  // s.unsubscribe();
  // subject.subscribe(|v| println!("Received 2: {v}"));

  // println!("---");

  // subject.next(700);
  // subject.next(800);
}
