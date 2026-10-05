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

function fixture(platform = 'android') {
  const filename = path.resolve(__dirname, '..', '..', '..', 'packages', 'react-native', 'vessel', 'src', 'index.tsx');
  const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
    filename,
    babelrc: false,
    configFile: false,
    presets: ['module:@react-native/babel-preset'],
  });
  const effects = [];
  const dimensions = { scale: 3, fontScale: 1.5 };
  const react = {
    forwardRef: render => render,
    useState: init => [init()],
    useEffect: (run, deps) => effects.push({ run, deps }),
  };
  const native = {
    Platform: { OS: platform },
    UIManager: {
      getViewManagerConfig: () => ({
        Constants:
          platform === 'ios'
            ? { defaultFontSize: 17, defaultFontPath: '/System/Library/Fonts/system.ttf' }
            : { defaultFontSize: 14 },
      }),
    },
    useWindowDimensions: () => dimensions,
    requireNativeComponent: () => 'NativeVesselView',
  };
  const module = { exports: {} };
  vm.runInNewContext(
    code,
    {
      module,
      exports: module.exports,
      require: name => {
        if (name === 'react') return react;
        if (name === 'react-native') return native;
        if (name.startsWith('@babel/runtime/')) return require(name);
        if (name === 'react/jsx-runtime') return { jsx: (type, props) => ({ type, props }) };
        throw new Error(`unexpected import ${name}`);
      },
    },
    { filename },
  );
  return {
    effects,
    dimensions,
    render: props => module.exports.VesselView(props),
  };
}

test('VesselView supplies platform font, density and accessibility scaling without props', () => {
  const f = fixture();
  const calls = [];
  const source = {
    setHostFont: (...args) => {
      calls.push(['font', ...args]);
      return true;
    },
    mount: id => calls.push(['mount', id]),
    unmount: id => calls.push(['unmount', id]),
  };
  const view = f.render({ source });
  f.effects[0].run();
  const cleanup = f.effects[1].run();
  assert.deepEqual(calls[0], ['font', undefined, 63]);
  assert.equal(calls[1][0], 'mount');
  assert.equal(view.props.fontSize, undefined, 'font props are not sent to the native View');
  cleanup();
  assert.equal(calls[2][0], 'unmount');
});

test('font overrides and scale changes update typography independently of mounting', () => {
  const f = fixture();
  const calls = [];
  const source = {
    setHostFont: (...args) => {
      calls.push(args);
      return true;
    },
  };
  f.render({ source, fontPath: 'custom.ttf', fontSize: 20, allowFontScaling: false });
  f.effects[0].run();
  assert.deepEqual(calls[0], ['custom.ttf', 60]);
  f.effects.length = 0;
  f.dimensions.fontScale = 2;
  f.render({ source, fontSize: 20 });
  f.effects[0].run();
  assert.deepEqual(calls[1], [undefined, 120]);
  assert.equal(f.effects[1].deps.length, 2, 'mounting does not depend on font settings');
});

test('iOS supplies its native system-font file and default size automatically', () => {
  const f = fixture('ios');
  const calls = [];
  f.render({
    source: {
      setHostFont: (...args) => {
        calls.push(args);
        return true;
      },
    },
  });
  f.effects[0].run();
  assert.deepEqual(calls[0], ['/System/Library/Fonts/system.ttf', 77]);
});

test('missing sources are safe and invalid fonts are reported explicitly', () => {
  const f = fixture();
  f.render({ source: null });
  assert.equal(f.effects[0].run(), undefined);
  assert.equal(f.effects[1].run(), undefined);
  assert.throws(() => f.render({ source: null, fontSize: NaN }), /finite and positive/);
  assert.throws(() => f.render({ source: null, fontSize: Number.MAX_VALUE }), /supported pixel range/);
  f.effects.length = 0;
  f.render({ source: { setHostFont: () => false }, fontPath: 'missing.ttf' });
  assert.throws(() => f.effects[0].run(), /could not load its host font/);
});
