import { AppRegistry } from 'react-native';
// Before anything loads the native bindings. This engine has no `FinalizationRegistry`, which they use to free a handle nobody released
// when its wrapper is garbage-collected, so this gives it one.
import './src/lib/native/finalization-registry';
// And this keeps a ledger of the native handles that are alive (development only).
import './src/lib/native/handle-ledger';
import App from './src/App';
import { name as appName } from './app.json';

AppRegistry.registerComponent(appName, () => App);
