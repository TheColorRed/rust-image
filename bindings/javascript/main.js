import ffi from 'ffi-napi';

const abra = ffi.Library('abra', {
  'color': {
    'black': ['pointer', []],
  }
});

const color_ptr = abra.color.black();

console.log('Color pointer:', color_ptr);