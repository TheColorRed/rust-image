use core::cell::RefCell;
use paste::paste;
use std::fs;

use saphyr::{LoadableYamlNode, Yaml};

/// Creates a getter method for each specified setting field.
/// - `getter` - The setting field name.
/// - `ret` - The return type of the getter.
///
/// ```ignore
/// yaml_settings_getters!(a => bool, b => String);
/// ```
macro_rules! yaml_settings_getters {
  // Accept comma separated list of `ident => type` pairs, with optional trailing comma.
  ( $( $getter:ident => $ret:ty ),* $(,)? ) => {
    $(
       #[doc = concat!("Gets the value of `", stringify!($getter), "`.")]
      pub fn $getter() -> $ret {
        if SETTINGS.with(|s| s.borrow().is_none()) {
          println!("Settings not initialized, initializing with default path.");
          Settings::init();
        }
        let result = SETTINGS.with(|s| s.borrow().as_ref().unwrap().settings.$getter.clone());
        println!("Settings: Getting '{}': {:?}", stringify!($getter), result);
        result
      }
      paste! {
        #[doc = concat!("Sets the value of `", stringify!($getter), "`.")]
        pub fn [<set_ $getter>](value: $ret) {
          if SETTINGS.with(|s| s.borrow().is_none()) {
            println!("Settings not initialized, initializing with default path.");
            Settings::init();
          }
          SETTINGS.with(|s| {
            if let Some(settings) = s.borrow_mut().as_mut() {
              println!("Settings: Setting '{}': {:?}", stringify!($getter), &value);
              settings.settings.$getter = value;
            }
          });
        }
      }
    )*
  };
}

thread_local! {
  static SETTINGS: RefCell<Option<Settings>> = RefCell::new(None);
}

#[derive(Clone)]
pub struct YamlSettings {
  gpu_enabled: bool,
  api_model_paths: Vec<String>,
}

#[derive(Clone)]
pub struct Settings {
  /// Enable or disable GPU acceleration globally.
  settings: YamlSettings,
}

impl Default for Settings {
  fn default() -> Self {
    Settings {
      settings: YamlSettings {
        gpu_enabled: true,
        api_model_paths: Vec::new(),
      },
    }
  }
}

impl Settings {
  /// Initialize settings using the default settings file path.
  pub fn init() -> Self {
    Self::init_from_file("./settings.yml")
  }
  /// Initialize settings from a specified file path.
  /// - `p_file` - The file path to load settings from.
  pub fn init_from_file(p_file: impl Into<String>) -> Self {
    let file = p_file.into();
    println!("Looking for settings file: {}", file);
    let settings_exist = fs::metadata(&file).is_ok();
    let result;
    if settings_exist {
      println!("Found settings file, loading...");
      let settings = fs::read_to_string(&file).unwrap();
      let docs = Yaml::load_from_str(&settings).unwrap();
      let doc = docs.get(0).unwrap();
      result = Self {
        settings: YamlSettings {
          gpu_enabled: doc
            .as_mapping_get("gpu")
            .and_then(|gpu| gpu.as_mapping_get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
          api_model_paths: doc
            .as_mapping_get("api")
            .and_then(|api| api.as_mapping_get("model_paths"))
            .and_then(|v| v.as_vec())
            .map(|v| v.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["packages/ai/models".to_string()]),
        },
        ..Default::default()
      };
    } else {
      println!("Settings file not found, using defaults.");
      result = Default::default();
    }
    SETTINGS.with(|s| s.replace(Some(result.clone())));
    result
  }

  yaml_settings_getters!(
    gpu_enabled => bool,
    api_model_paths => Vec<String>
  );
}
