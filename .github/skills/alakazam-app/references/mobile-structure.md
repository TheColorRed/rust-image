# Structure of the Alakazam Mobile App

The Alakazam mobile app is a React Native application in `/apps/alakazam/mobile`. Its image-manipulation bindings live in the `@alakazam/mobile` package in `/packages/react-native/alakazam`. Below is an overview of the main components:

```text
apps/alakazam/mobile/
├── package.json                    # React Native app configuration and scripts
├── app.json                        # Application name and display name
├── index.js                        # React Native application entry point
├── babel.config.js                 # Babel configuration
├── metro.config.js                 # Metro bundler configuration
├── global.css                      # Global NativeWind styles
├── tailwind.config.js              # Tailwind/NativeWind configuration
├── tsconfig.json                   # TypeScript configuration
├── android/                        # Android project and native integrations
├── assets/                         # Application assets
└── src/
    ├── App.tsx                     # Root application component and navigation state
    ├── components/                 # Reusable UI components
    │   └── tools/                  # Tool-specific UI components
    ├── hooks/                      # Custom React hooks
    ├── lib/                        # Theme, styling, media, and utility modules
    └── screens/                    # Home, editor, camera, settings, and Abra screens

packages/react-native/alakazam/
├── package.json                    # @alakazam/mobile package configuration
├── Cargo.toml                      # Rust crate configuration for the mobile bindings
├── ubrn.config.yaml                # uniffi-bindgen-react-native configuration
├── AlakazamMobile.podspec          # Generated iOS CocoaPods integration
├── android/                        # Generated Android React Native integration
├── scripts/
│   └── build-android.mjs           # Builds Android libraries and regenerates bindings
└── src/
    ├── lib.rs                      # UniFFI crate entry point
    ├── image.rs                    # Abra image binding API
    ├── adjustments.rs               # Image adjustment binding API
    ├── filters.rs                   # Image filter binding API
    ├── tools.rs                     # Image tool binding API
    ├── transforms.rs                # Image transform binding API
    ├── history.rs                   # Edit-history binding API
    ├── index.tsx                   # React Native binding package entry point
    └── generated/                  # Generated UniFFI TypeScript bindings
```
