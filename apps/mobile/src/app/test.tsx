import { StyleSheet, Text, View } from 'react-native';

export default function Test() {
  return (
    <View style={styles.container}>
      <Text style={styles.title}>Test Screen</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  title: { fontSize: 20, fontWeight: 'bold' },
});
