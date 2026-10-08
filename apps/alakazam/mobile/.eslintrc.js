module.exports = {
  root: true,
  extends: ['@react-native', 'plugin:react/recommended', 'prettier'],
  parser: '@typescript-eslint/parser',
  plugins: ['@typescript-eslint', 'react', 'react-native'],
  env: { es2021: true, node: true, browser: true },
  settings: { react: { version: 'detect' } },
  rules: {
    'react/jsx-uses-react': 'off',
    'react/react-in-jsx-scope': 'off',
  },
};
