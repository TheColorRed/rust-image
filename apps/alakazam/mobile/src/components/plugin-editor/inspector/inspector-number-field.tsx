import { useTheme } from '@/src/lib/ui/theme';
import { useEffect, useState } from 'react';
import { StyleSheet, TextInput } from 'react-native';

/** How many decimals a step shows: 1 for 1, 1 for 0.1, 2 for 0.05. */
const decimalsOf = (step: number) => Math.min(6, (String(step).split('.')[1] ?? '').length);

/**
 * A number typed in, at the end of a slider as in the Unity inspector. It keeps what is typed until the field is left, so it can be
 * emptied to type a new number, and it tells the caller only when what is typed is a number. When the number changes from outside
 * (the slider moved) the field shows it.
 */
export function InspectorNumberField({
  value,
  step,
  label,
  fill = false,
  onChange,
}: {
  value: number;
  step: number;
  /** What a screen reader calls the field. */
  label: string;
  /** Fills the width it is given, as a field of a row does, instead of the fixed width it has at the end of a slider. */
  fill?: boolean;
  /** Called with the number each time what is typed is one. */
  onChange: (value: number) => void;
}) {
  const { colors } = useTheme();
  const shown = String(Number(value.toFixed(decimalsOf(step))));
  const [text, setText] = useState(shown);
  const [typing, setTyping] = useState(false);
  useEffect(() => {
    if (!typing) setText(shown);
  }, [shown, typing]);

  return (
    <TextInput
      accessibilityLabel={label}
      value={text}
      onFocus={() => setTyping(true)}
      onBlur={() => {
        setTyping(false);
        setText(shown);
      }}
      onChangeText={next => {
        setText(next);
        const number = next.trim() === '' ? NaN : Number(next);
        if (!Number.isNaN(number)) onChange(number);
      }}
      keyboardType="numeric"
      selectTextOnFocus
      style={[
        styles.input,
        fill && styles.fill,
        { color: colors.textPrimary, backgroundColor: colors.chip, borderColor: colors.border },
      ]}
    />
  );
}

const styles = StyleSheet.create({
  input: {
    width: 56,
    height: 26,
    paddingHorizontal: 6,
    paddingVertical: 0,
    borderWidth: StyleSheet.hairlineWidth,
    borderRadius: 6,
    fontSize: 12,
    textAlign: 'center',
  },
  fill: { width: '100%', paddingHorizontal: 8, textAlign: 'left' },
});
