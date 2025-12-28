// webpack.config.js
import ReactRefreshWebpackPlugin from '@pmmmwh/react-refresh-webpack-plugin';
import HtmlWebpackPlugin from 'html-webpack-plugin';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import webpack from 'webpack';

const __dirname = dirname(fileURLToPath(import.meta.url));
const isDevelopment = process.env.NODE_ENV !== 'production';

const rendererConfig = {
  name: 'renderer',
  mode: isDevelopment ? 'development' : 'production',
  entry: {
    renderer: './src/renderer/renderer.tsx',
    dialog: './src/renderer/dialogs/entry.tsx',
  },
  output: {
    path: resolve(__dirname, 'dist/renderer'),
    filename: '[name].bundle.js',
  },
  module: {
    rules: [
      {
        test: /\.(js|jsx|ts|tsx)$/,
        exclude: /node_modules/,
        use: {
          loader: 'babel-loader',
          options: {
            presets: [['@babel/preset-react', { runtime: 'automatic' }], '@babel/preset-typescript'],
            plugins: [isDevelopment && 'react-refresh/babel'].filter(Boolean),
          },
        },
      },
      {
        test: /\.css$/,
        use: ['style-loader', 'css-loader', 'postcss-loader'],
      },
      {
        test: /\.js$/,
        resolve: {
          fullySpecified: false,
        },
      },
    ],
  },
  resolve: {
    extensions: ['.js', '.jsx', '.ts', '.tsx'],
    alias: {
      '@': resolve(__dirname, 'src/renderer'),
    },
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: './src/server/index.html',
      chunks: ['renderer'],
    }),
    new HtmlWebpackPlugin({
      template: './src/server/dialog.html',
      filename: 'dialog.html',
      chunks: ['dialog'],
    }),
    new webpack.DefinePlugin({
      global: 'globalThis',
    }),
    isDevelopment && new ReactRefreshWebpackPlugin(),
  ].filter(Boolean),
  target: 'web',
  devServer: {
    port: 8080,
    hot: true,
    static: {
      directory: resolve(__dirname, 'dist'),
    },
    client: {
      overlay: true,
      progress: true,
    },
    liveReload: false, // Disable live reload to reduce polling (HMR still works)
    headers: {
      'Content-Security-Policy':
        "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com data:; img-src 'self' data: blob:; connect-src 'self' ws://localhost:*",
    },
  },
};

const serverConfig = {
  name: 'server',
  mode: isDevelopment ? 'development' : 'production',
  entry: './src/server/main.ts',
  output: {
    path: resolve(__dirname, 'dist/server'),
    filename: 'main.js',
  },
  resolve: {
    extensions: ['.js', '.jsx', '.ts', '.tsx'],
    alias: {
      '@': resolve(__dirname, 'src/server'),
    },
  },
  module: {
    rules: [
      {
        test: /\.(js|ts)$/,
        exclude: /node_modules/,
        use: {
          loader: 'babel-loader',
          options: {
            presets: ['@babel/preset-typescript'],
          },
        },
      },
    ],
  },
  externals: {
    electron: 'commonjs electron',
    '@alakazam/abra/abra.node': 'commonjs @alakazam/abra/abra.node',
    '@alakazam/history/alakazam-history.node': 'commonjs @alakazam/history/alakazam-history.node',
  },
  target: 'electron-main',
};

const preloadConfig = {
  name: 'preload',
  mode: isDevelopment ? 'development' : 'production',
  entry: './src/server/preload/preload.ts',
  output: {
    path: resolve(__dirname, 'dist/server'),
    filename: 'preload.js',
  },
  module: {
    rules: [
      {
        test: /\.(js|ts)$/,
        exclude: /node_modules/,
        use: {
          loader: 'babel-loader',
          options: {
            presets: ['@babel/preset-typescript'],
          },
        },
      },
    ],
  },
  resolve: {
    extensions: ['.js', '.ts'],
  },
  externals: {
    electron: 'commonjs electron',
  },
  target: 'electron-preload',
};

export default [rendererConfig, serverConfig, preloadConfig];
