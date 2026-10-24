import { APP_SLOGAN } from '@/src/lib/brand';
import { useTheme } from '@/src/lib/ui/theme';
import { useEffect, useMemo, useState } from 'react';
import { Animated, Easing, Text, View, type LayoutChangeEvent } from 'react-native';
import Svg, { Circle, Defs, LinearGradient, Path, Stop, Text as SvgText } from 'react-native-svg';

/** Renders the shared slogan in a bright, multicolor spell-like gradient with a few star glints. */
export function MagicalSlogan({ fontSize = 24, subText }: { fontSize?: number; subText?: string }) {
  const [width, setWidth] = useState(0);
  const [textWidth, setTextWidth] = useState(0);
  const { colors } = useTheme();
  const height = fontSize * 1.7;
  const onLayout = (event: LayoutChangeEvent) => setWidth(event.nativeEvent.layout.width);
  const sparkles = useMemo(
    () =>
      Array.from({ length: 8 }, (_, index) => {
        const size = 1.1 + Math.random() * 4;
        const verticalMin = fontSize * 0.18 + size * 2;
        const verticalRange = Math.max(1, fontSize - size * 4);
        return {
          id: index,
          side: index % 2 === 0 ? -1 : 1,
          distance: 3 + Math.random() * 10,
          size,
          top: verticalMin + Math.random() * verticalRange,
          color: ['#fff4ad', '#a5faff', '#ff9ee8', '#ffffff'][Math.floor(Math.random() * 4)],
          star: Math.random() > 0.5,
          phase: new Animated.Value(0),
          xFrequency: 1 + Math.floor(Math.random() * 3),
          xFrequency2: 2 + Math.floor(Math.random() * 4),
          yFrequency: 1 + Math.floor(Math.random() * 3),
          yFrequency2: 2 + Math.floor(Math.random() * 4),
          xPhase: Math.random() * Math.PI * 2,
          xPhase2: Math.random() * Math.PI * 2,
          yPhase: Math.random() * Math.PI * 2,
          yPhase2: Math.random() * Math.PI * 2,
          opacityPhase: Math.random() * Math.PI * 2,
          opacityFrequency: 1 + Math.floor(Math.random() * 3),
          rotationTurns: (1 + Math.floor(Math.random() * 3)) * (Math.random() > 0.5 ? 1 : -1),
          driftX: 12 + Math.random() * 18,
          verticalMin,
          verticalRange,
          cycleDuration: 10000 + Math.random() * 12000,
          startDelay: Math.random() * 1200,
        };
      }),
    [fontSize],
  );

  useEffect(() => {
    const timers: ReturnType<typeof setTimeout>[] = [];
    const loops: Animated.CompositeAnimation[] = [];
    sparkles.forEach(sparkle => {
      timers.push(
        setTimeout(() => {
          const loop = Animated.loop(
            Animated.timing(sparkle.phase, {
              toValue: 1,
              duration: sparkle.cycleDuration,
              easing: Easing.linear,
              useNativeDriver: true,
            }),
          );
          loops.push(loop);
          loop.start();
        }, sparkle.startDelay),
      );
    });
    return () => {
      timers.forEach(clearTimeout);
      loops.forEach(loop => loop.stop());
      sparkles.forEach(sparkle => sparkle.phase.stopAnimation());
    };
  }, [sparkles]);

  return (
    <View onLayout={onLayout} style={{ width: '100%', height, alignItems: 'center', justifyContent: 'center' }}>
      <Text
        accessible={false}
        onLayout={event => setTextWidth(event.nativeEvent.layout.width)}
        style={{
          position: 'absolute',
          alignSelf: 'center',
          opacity: 0,
          fontFamily: 'sans-serif',
          fontSize,
          fontWeight: '900',
          includeFontPadding: false,
        }}
      >
        {APP_SLOGAN}
      </Text>
      {width > 0 && (
        <Svg width={width} height={height} viewBox={`0 0 ${width} ${height}`}>
          <Defs>
            <LinearGradient id="slogan-magic" x1="0" y1="0" x2="1" y2="0">
              <Stop offset="0%" stopColor="#53f4ff" />
              <Stop offset="28%" stopColor="#9e7bff" />
              <Stop offset="57%" stopColor="#ff74df" />
              <Stop offset="82%" stopColor="#ffd86a" />
              <Stop offset="100%" stopColor="#7fffc0" />
            </LinearGradient>
          </Defs>
          <SvgText
            x={width / 2}
            y={fontSize * 1.18}
            textAnchor="middle"
            fontSize={fontSize}
            fontWeight="900"
            fontFamily="sans-serif"
            fill="url(#slogan-magic)"
          >
            {APP_SLOGAN}
          </SvgText>
        </Svg>
      )}
      {width > 0 &&
        textWidth > 0 &&
        sparkles.map(sparkle => {
          const rawX = width / 2 + sparkle.side * (textWidth / 2 + sparkle.distance);
          const x = Math.max(sparkle.size * 2, Math.min(width - sparkle.size * 2, rawX));
          const samples = 48;
          const inputRange = Array.from({ length: samples + 1 }, (_, index) => index / samples);
          const yAmplitude = Math.max(
            0,
            Math.min(
              fontSize * 0.42,
              sparkle.top - sparkle.verticalMin,
              sparkle.verticalMin + sparkle.verticalRange - sparkle.top,
            ),
          );
          const relativeWave = (time: number, frequency: number, phase: number) =>
            Math.sin(2 * Math.PI * frequency * time + phase) - Math.sin(phase);
          const xRange = Math.min(
            sparkle.driftX,
            Math.max(1, Math.min(rawX - sparkle.size * 2, width - rawX - sparkle.size * 2)),
          );
          const xValues = inputRange.map(
            time =>
              xRange *
              0.5 *
              (0.68 * relativeWave(time, sparkle.xFrequency, sparkle.xPhase) +
                0.32 * relativeWave(time, sparkle.xFrequency2, sparkle.xPhase2)),
          );
          const yValues = inputRange.map(
            time =>
              yAmplitude *
              0.5 *
              (0.68 * relativeWave(time, sparkle.yFrequency, sparkle.yPhase) +
                0.32 * relativeWave(time, sparkle.yFrequency2, sparkle.yPhase2)),
          );
          const opacityValues = inputRange.map(
            time =>
              0.72 +
              0.28 * (0.5 + 0.5 * Math.sin(2 * Math.PI * sparkle.opacityFrequency * time + sparkle.opacityPhase)),
          );
          const translateX = sparkle.phase.interpolate({ inputRange, outputRange: xValues });
          const translateY = sparkle.phase.interpolate({ inputRange, outputRange: yValues });
          const opacity = sparkle.phase.interpolate({ inputRange, outputRange: opacityValues });
          const rotate = sparkle.phase.interpolate({
            inputRange: [0, 1],
            outputRange: ['0deg', `${sparkle.star ? sparkle.rotationTurns * 360 : 0}deg`],
          });
          return (
            <Animated.View
              key={sparkle.id}
              pointerEvents="none"
              style={{
                position: 'absolute',
                left: x - sparkle.size * 2,
                top: sparkle.top - sparkle.size * 2,
                width: sparkle.size * 4,
                height: sparkle.size * 4,
                alignItems: 'center',
                justifyContent: 'center',
                opacity,
                transform: [{ translateX }, { translateY }, { rotate }],
              }}
            >
              <Svg width={sparkle.size * 4} height={sparkle.size * 4} viewBox="0 0 4 4">
                {sparkle.star ? (
                  <Path d="M2 0 L2.55 1.45 L4 2 L2.55 2.55 L2 4 L1.45 2.55 L0 2 L1.45 1.45 Z" fill={sparkle.color} />
                ) : (
                  <Circle cx={2} cy={2} r={1.1} fill={sparkle.color} />
                )}
              </Svg>
            </Animated.View>
          );
        })}
      {subText && (
        <Text style={{ color: colors.textMuted, textAlign: 'center', fontSize: 20, fontWeight: '600' }}>{subText}</Text>
      )}
    </View>
  );
}
