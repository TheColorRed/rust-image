const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const babel = require(
  require.resolve('@babel/core', {
    paths: [path.dirname(require.resolve('@react-native/babel-preset'))],
  }),
);

function load(platform) {
  const filename = path.join(__dirname, '..', 'src', 'index.tsx');
  const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
    filename,
    babelrc: false,
    configFile: false,
    presets: ['module:@react-native/babel-preset'],
  });
  const effects = [];
  const requested = [];
  let viewId;
  const createElement = (type, props) => ({ type, props });
  const react = {
    forwardRef: render => render,
    useEffect: effect => effects.push(effect),
    useState: initial => {
      if (viewId === undefined) viewId = initial();
      return [viewId];
    },
    createElement,
  };
  const modules = {
    react,
    'react/jsx-runtime': { jsx: createElement, jsxs: createElement },
    'react-native': {
      Platform: { OS: platform },
      UIManager: {
        getViewManagerConfig: () => ({ Constants: { defaultFontSize: platform === 'ios' ? 17 : 14 } }),
      },
      useWindowDimensions: () => ({ scale: 2, fontScale: 1 }),
      requireNativeComponent(name) {
        requested.push(name);
        return 'NativeVesselView';
      },
    },
  };
  const module = { exports: {} };
  vm.runInNewContext(code, {
    module,
    exports: module.exports,
    require: name => {
      if (name.startsWith('@babel/runtime/helpers/')) return require(name);
      assert.ok(name in modules, `unexpected import: ${name}`);
      return modules[name];
    },
  }, { filename });
  return { ...module.exports, effects, requested };
}

for (const platform of ['android', 'ios']) {
  test(`${platform} registers the native view and mounts and unmounts its source`, () => {
    const { VesselView, effects, requested } = load(platform);
    assert.deepEqual(requested, ['VesselView']);
    const calls = [];
    const source = {
      setHostFont: () => true,
      mount: id => calls.push(['mount', id]),
      unmount: id => calls.push(['unmount', id]),
    };
    const style = { width: 112, height: 80 };
    const rendered = VesselView({ source, style, pointerEvents: 'none' }, null);
    assert.equal(rendered.props.style, style);
    assert.equal(rendered.props.pointerEvents, 'none');
    assert.ok(rendered.props.surfaceId > 0);
    effects[0]();
    const cleanup = effects[1]();
    assert.deepEqual(calls, [['mount', rendered.props.surfaceId]]);
    cleanup();
    assert.deepEqual(calls[1], ['unmount', rendered.props.surfaceId]);
  });

  test(`${platform} can replace its source without changing the native view id`, () => {
    const { VesselView, effects } = load(platform);
    const calls = [];
    const source = name => ({
      setHostFont: () => true,
      mount: id => calls.push([name, 'mount', id]),
      unmount: id => calls.push([name, 'unmount', id]),
    });
    const first = VesselView({ source: source('first') }, null);
    effects[0]();
    const cleanup = effects[1]();
    const second = VesselView({ source: source('second') }, null);
    cleanup();
    effects[2]();
    effects[3]();
    assert.equal(first.props.surfaceId, second.props.surfaceId);
    assert.deepEqual(calls.map(call => call.slice(0, 2)), [
      ['first', 'mount'], ['first', 'unmount'], ['second', 'mount'],
    ]);
  });

  test(`${platform} accepts a null source without attempting to mount it`, () => {
    const { VesselView, effects } = load(platform);
    const rendered = VesselView({ source: null }, null);
    assert.equal(rendered.type, 'NativeVesselView');
    assert.equal(effects[0](), undefined);
    assert.equal(effects[1](), undefined);
  });
}

test('unsupported platforms do not request a native Vessel view', () => {
  const { VesselView, requested } = load('web');
  assert.equal(VesselView, null);
  assert.deepEqual(requested, []);
});
