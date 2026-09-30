# Structure of the Alakazam App

The Alakazam App is broken down into several key directories and files that make up the application. Below is an overview of the main components:

```
apps/alakazam/
├── package.json             # Node.js project configuration
├── tsconfig.json            # TypeScript configuration
├── webpack.config.mjs       # Webpack bundler configuration
├── postcss.config.mjs       # PostCSS configuration
├── scripts/                 # Build scripts
│   ├── move-bindings.mjs    # Script to move Rust bindings
│   └── write-bindings-ready.mjs
├── src/
│   ├── global.d.ts          # Global TypeScript declarations
│   ├── renderer/            # Frontend React application
│   │   ├── renderer.tsx     # React entry point
│   │   ├── app.tsx          # Main React app component
│   │   ├── components/      # Reusable React components
│   │   ├── pages/           # Page-level components
│   │   ├── dialogs/         # Dialog components
│   │   ├── state/           # State management
│   │   ├── services/        # Frontend services
│   │   ├── hooks/           # Custom React hooks
│   │   ├── ui/              # UI components
│   │   └── lib/             # Utility libraries
│   └── server/              # Electron main process
│       ├── main.ts          # Electron main entry point
│       ├── preload/         # Preload scripts for IPC
│       ├── actions/         # Main process actions
│       ├── events/          # Event handlers
│       └── services/        # Backend services
└── dist/                    # Built application output
```
