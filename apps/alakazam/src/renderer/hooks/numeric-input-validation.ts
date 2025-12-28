import { useCallback } from 'react';

const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
  // if the keyboard event is Space, then ignore the input
  if (e.key === ' ') {
    e.preventDefault();
    return;
  }
  // Allow common editing and navigation keys
  const allowed = ['Backspace', 'Delete', 'ArrowLeft', 'ArrowRight', 'Home', 'End', 'Tab', 'Enter'];
  if (allowed.includes(e.key)) return;
  if (e.ctrlKey || e.metaKey || e.altKey) return; // allow copy/paste and other combos
  // Allow digits
  if (/^[0-9]$/.test(e.key)) return;

  const input = e.currentTarget as HTMLInputElement;
  const val = input.value ?? '';
  const selStart = input.selectionStart ?? 0;
  const selEnd = input.selectionEnd ?? selStart;
  const dashIndex = val.indexOf('-');
  const dotIndex = val.indexOf('.');

  // '-' can only be present once and only at the beginning (index 0),
  // but if the selection includes the existing '-' allow replacement.
  if (e.key === '-') {
    if (dashIndex === -1) {
      if (selStart !== 0) e.preventDefault();
    } else {
      if (!(selStart <= dashIndex && selEnd > dashIndex)) e.preventDefault();
    }
    return;
  }

  // '.' can only appear once and cannot be placed before an existing '-' unless
  // the selection includes the '-' (replacement).
  if (e.key === '.') {
    if (dotIndex !== -1 && !(selStart <= dotIndex && selEnd > dotIndex)) {
      e.preventDefault();
      return;
    }
    if (dashIndex !== -1 && !(selStart <= dashIndex && selEnd > dashIndex) && selStart <= dashIndex) {
      e.preventDefault();
      return;
    }
    return;
  }

  e.preventDefault();
};

const onPaste = (e: React.ClipboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
  const paste = e.clipboardData?.getData('text') ?? '';
  const input = e.currentTarget as HTMLInputElement;
  const { value } = input;
  const selStart = input.selectionStart ?? 0;
  const selEnd = input.selectionEnd ?? selStart;
  const newValue = value.substring(0, selStart) + paste + value.substring(selEnd);
  // Validate against the same rules: only one optional leading '-', only one '.', and '.' cannot be before '-'
  const dashIndex = newValue.indexOf('-');
  if (newValue.split('-').length > 2 || (dashIndex > 0 && newValue.includes('-'))) {
    e.preventDefault();
    return;
  }
  const dotIndex = newValue.indexOf('.');
  if (newValue.split('.').length > 2) {
    e.preventDefault();
    return;
  }
  if (dotIndex !== -1 && dashIndex !== -1 && dotIndex < dashIndex) {
    e.preventDefault();
    return;
  }
  // Also validate against the numeric input regex (allow intermediate states)
  if (!/^-?\d*\.?\d*$/.test(newValue)) e.preventDefault();
};

export function useNumericInputValidation() {
  return { onKeyDown: useCallback(onKeyDown, []), onPaste: useCallback(onPaste, []) };
}
