import { useEffect, useMemo, useRef, useState } from 'react';
import { Animated, Easing, StyleSheet, View } from 'react-native';
import { useObservable } from 'react-rx';
import Svg, { Defs, LinearGradient, Rect, Stop } from 'react-native-svg';
import { useTheme } from '@/src/lib/theme';
import { aiProgress, aiProgress$, overallProgress } from '@/src/state/ai-progress';

const HEIGHT = 4;
/** The colors the bar flows through, in the order they follow each other: blue, violet, pink. Stronger than a soft banner so a thin line shows. */
const FLOW = ['#4F8CFF', '#8B5CF6', '#FF6FB5'];
/** How long the colors take to flow once across the bar. */
const FLOW_MS = 2400;
/** How long the bar takes to fade down and up again, while part of what it shows is a guess. */
const PULSE_MS = 900;
/**
 * A task that does not say how far along it is is shown as a guess: it moves on quickly at first and ever more slowly, and never gets
 * past `GUESS_LIMIT` until the task ends. `GUESS_MS` is how long it takes to cover about two thirds of that.
 */
const GUESS_LIMIT = 0.9;
const GUESS_MS = 3000;

/**
 * A thin bar between the photo and the panel that shows while an AI model runs: a gradient that keeps flowing sideways. It always starts
 * empty at the left and fills toward the right. It fills as far as the tasks say they have got; a task that does not say is guessed
 * from the time it has been running, and the bar pulses while any of it is a guess. It always takes its height, empty when nothing runs,
 * so the photo does not change size when it appears (a new size would draw the photo again).
 */
export function AiProgressBar() {
  const { colors } = useTheme();
  const tasks = useObservable(aiProgress$, aiProgress.value);
  const running = Object.keys(tasks).length > 0;
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    setElapsed(0);
    if (!running) return;
    const started = Date.now();
    const timer = setInterval(() => setElapsed(Date.now() - started), 100);
    return () => clearInterval(timer);
  }, [running]);
  const { share, unsure } = overallProgress(tasks, GUESS_LIMIT * (1 - Math.exp(-elapsed / GUESS_MS)));
  // The bar only moves forward during a job: a task that starts late adds to the whole and would otherwise pull it back.
  const furthest = useRef(0);
  furthest.current = running ? Math.max(furthest.current, share) : 0;
  const progress = furthest.current;
  const [width, setWidth] = useState(0);
  const filled = useRef(new Animated.Value(0)).current;
  const flow = useRef(new Animated.Value(0)).current;
  const pulse = useRef(new Animated.Value(1)).current;
  const styles = useMemo(
    () =>
      StyleSheet.create({
        track: { height: HEIGHT, borderRadius: HEIGHT / 2, overflow: 'hidden', backgroundColor: running ? colors.chip : 'transparent' },
        clip: { height: HEIGHT, overflow: 'hidden' },
      }),
    [colors, running],
  );

  useEffect(() => {
    // A new job starts from nothing, not from where the last one ended.
    if (!running) filled.setValue(0);
    else Animated.timing(filled, { toValue: progress, duration: 200, easing: Easing.out(Easing.ease), useNativeDriver: false }).start();
  }, [filled, progress, running]);

  useEffect(() => {
    if (!running) return;
    flow.setValue(0);
    const loop = Animated.loop(Animated.timing(flow, { toValue: 1, duration: FLOW_MS, easing: Easing.linear, useNativeDriver: true }));
    loop.start();
    return () => loop.stop();
  }, [running, flow]);

  const pulsing = running && unsure;
  useEffect(() => {
    pulse.setValue(1);
    if (!pulsing) return;
    const loop = Animated.loop(
      Animated.sequence([
        Animated.timing(pulse, { toValue: 0.45, duration: PULSE_MS, easing: Easing.inOut(Easing.ease), useNativeDriver: true }),
        Animated.timing(pulse, { toValue: 1, duration: PULSE_MS, easing: Easing.inOut(Easing.ease), useNativeDriver: true }),
      ]),
    );
    loop.start();
    return () => loop.stop();
  }, [pulsing, pulse]);

  // Two lengths of the colors side by side. Sliding it by one length lands on the same colors it started on, so the loop has no seam.
  const stops = [...FLOW, ...FLOW, FLOW[0]];

  return (
    <View
      style={styles.track}
      onLayout={event => setWidth(event.nativeEvent.layout.width)}
      accessibilityRole="progressbar"
      accessibilityLabel="AI is working"
      accessibilityValue={running ? { min: 0, max: 100, now: Math.round(progress * 100) } : undefined}
    >
      {running && width > 0 ? (
        <Animated.View style={{ opacity: pulse }}>
          <Animated.View style={[styles.clip, { width: filled.interpolate({ inputRange: [0, 1], outputRange: [0, width] }) }]}>
            <Animated.View
              style={{ width: width * 2, transform: [{ translateX: flow.interpolate({ inputRange: [0, 1], outputRange: [0, -width] }) }] }}
            >
              <Svg width={width * 2} height={HEIGHT}>
                <Defs>
                  <LinearGradient id="ai-flow" x1="0" y1="0" x2="1" y2="0">
                    {stops.map((color, index) => (
                      <Stop key={index} offset={index / (stops.length - 1)} stopColor={color} />
                    ))}
                  </LinearGradient>
                </Defs>
                <Rect width={width * 2} height={HEIGHT} fill="url(#ai-flow)" />
              </Svg>
            </Animated.View>
          </Animated.View>
        </Animated.View>
      ) : null}
    </View>
  );
}
