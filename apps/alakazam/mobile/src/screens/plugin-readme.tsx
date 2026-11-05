import { useCommonStyles } from '@/src/lib/ui/styles';
import { useTheme } from '@/src/lib/ui/theme';
import { errorMessage } from '@/src/lib/utils/error-message';
import { readPluginDocument, type PluginInfo } from '@alakazam/mobile';
import ChevronLeft from 'lucide-react-native/icons/chevron-left';
import { useEffect, useMemo, useState } from 'react';
import { ActivityIndicator, Image, Linking, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';

type Props = {
  name: string;
  /** From the catalog, for a plugin that is not installed. */
  summary?: string;
  author?: string;
  version?: string;
  /** The catalog URL of the plugin's plugin.yml, for a plugin that is not installed. */
  manifestUrl?: string;
  /** The installed plugin, which has its documents, screenshots and contributions on this device. */
  installed?: PluginInfo;
  /** The name and description of a plugin this one depends on, when the app knows it (installed, or listed in the catalog). */
  describe?: (id: string) => { name: string; description: string } | undefined;
  onBack: () => void;
};

/** A document of the plugin: the installed copy wins (a plugin synced from the dev server shows the file being edited), otherwise the catalog's
 * copy beside its plugin.yml. `null` when the plugin ships none (404 or 403), and `undefined` while loading. */
function useDocument(p_localPath: string | undefined, p_manifestUrl: string | undefined, p_file: string) {
  const [text, setText] = useState<string | null | undefined>(undefined);
  const [failure, setFailure] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        if (p_localPath) return readPluginDocument(p_localPath);
        if (!p_manifestUrl) return null;
        const response = await fetch(p_manifestUrl.replace(/[^/]*$/, p_file), { cache: 'no-store' });
        if (response.status === 404 || response.status === 403) return null;
        if (!response.ok) throw new Error(`The server answered ${response.status}`);
        return await response.text();
      } catch (e) {
        if (!cancelled) setFailure(errorMessage(e));
        return null;
      }
    })().then(value => !cancelled && setText(value));
    return () => {
      cancelled = true;
    };
  }, [p_localPath, p_manifestUrl, p_file]);
  return { text, failure };
}

/** The store page of a plugin: its summary, screenshots, overview (README.md), what it adds, requirements and release notes (CHANGELOG.md). */
export function PluginReadmeScreen({ name, summary, author: listedAuthor, version, manifestUrl, installed, describe, onBack }: Props) {
  const { colors } = useTheme();
  const commonStyles = useCommonStyles();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: { flex: 1, backgroundColor: colors.background, paddingTop: 24 },
        title: { flex: 1, fontSize: 20, fontWeight: 'bold', color: colors.textPrimary },
        back: { width: 40, height: 40, alignItems: 'center', justifyContent: 'center' },
        content: { paddingHorizontal: 16, paddingTop: 12, paddingBottom: 48, gap: 8 },
        meta: { fontSize: 13, color: colors.textSecondary },
        tabs: { flexDirection: 'row', borderBottomWidth: 1, borderBottomColor: colors.border, marginTop: 12, marginBottom: 4 },
        tab: { flex: 1, alignItems: 'center', paddingVertical: 10, borderBottomWidth: 2, borderBottomColor: 'transparent' },
        tabText: { fontSize: 13, fontWeight: '600', color: colors.textSecondary },
        shots: { gap: 8 },
        shot: { height: 200, aspectRatio: 16 / 9, borderRadius: 8, backgroundColor: colors.border },
        tags: { flexDirection: 'row', flexWrap: 'wrap', gap: 6 },
        tag: { fontSize: 12, color: colors.textSecondary, paddingHorizontal: 8, paddingVertical: 3, borderRadius: 10, borderWidth: 1, borderColor: colors.border },
        h1: { fontSize: 22, fontWeight: '700', color: colors.textPrimary, marginTop: 8 },
        h2: { fontSize: 18, fontWeight: '600', color: colors.textPrimary, marginTop: 8 },
        h3: { fontSize: 16, fontWeight: '600', color: colors.textPrimary, marginTop: 4 },
        text: { fontSize: 15, lineHeight: 22, color: colors.textPrimary },
        bullet: { flexDirection: 'row', gap: 8, paddingLeft: 8 },
        code: { fontFamily: 'monospace', fontSize: 13, color: colors.textPrimary, padding: 12, backgroundColor: colors.border },
        empty: { fontSize: 15, color: colors.textSecondary },
      }),
    [colors],
  );

  const tabs = ['Overview', 'Package Content', 'Releases', 'Dependencies'] as const;
  const [tab, setTab] = useState<(typeof tabs)[number]>('Overview');
  const overview = useDocument(installed?.readmePath, manifestUrl, 'README.md');
  const notes = useDocument(installed?.changelogPath, manifestUrl, 'CHANGELOG.md');

  // Bold, italic, inline code and links inside a line of text.
  const inline = (p_text: string): React.ReactNode[] =>
    p_text.split(/(\*\*[^*]+\*\*|`[^`]+`|\*[^*]+\*|\[[^\]]+\]\([^)]+\))/).map((part, index) => {
      if (part.startsWith('**') && part.endsWith('**') && part.length > 4) return <Text key={index} style={{ fontWeight: '700' }}>{part.slice(2, -2)}</Text>;
      if (part.startsWith('`') && part.endsWith('`') && part.length > 2) return <Text key={index} style={{ fontFamily: 'monospace', backgroundColor: colors.border }}>{part.slice(1, -1)}</Text>;
      if (part.startsWith('*') && part.endsWith('*') && part.length > 2) return <Text key={index} style={{ fontStyle: 'italic' }}>{part.slice(1, -1)}</Text>;
      const link = part.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      if (link) return <Text key={index} style={{ color: colors.accent }} onPress={() => Linking.openURL(link[2]).catch(() => {})}>{link[1]}</Text>;
      return part;
    });

  // Headings, bullets, numbered lists, fenced code and paragraphs.
  const markdown = (p_text: string) => {
    const blocks: React.ReactNode[] = [];
    let fence: string[] | null = null;
    let paragraph: string[] = [];
    const endParagraph = () => {
      if (paragraph.length) blocks.push(<Text key={blocks.length} style={styles.text}>{inline(paragraph.join(' '))}</Text>);
      paragraph = [];
    };
    for (const line of p_text.replace(/\r\n/g, '\n').split('\n')) {
      if (line.trimStart().startsWith('```')) {
        if (fence) {
          blocks.push(<Text key={blocks.length} selectable style={styles.code}>{fence.join('\n')}</Text>);
          fence = null;
        } else {
          endParagraph();
          fence = [];
        }
      } else if (fence) {
        fence.push(line);
      } else if (/^#{1,6}\s/.test(line)) {
        endParagraph();
        const level = line.match(/^#+/)![0].length;
        blocks.push(<Text key={blocks.length} style={level === 1 ? styles.h1 : level === 2 ? styles.h2 : styles.h3}>{inline(line.replace(/^#+\s+/, ''))}</Text>);
      } else if (/^\s*([-*]|\d+\.)\s/.test(line)) {
        endParagraph();
        const number = line.match(/^\s*(\d+)\./)?.[1];
        blocks.push(
          <View key={blocks.length} style={styles.bullet}>
            <Text style={styles.text}>{number ? `${number}.` : '•'}</Text>
            <Text style={[styles.text, { flex: 1 }]}>{inline(line.replace(/^\s*([-*]|\d+\.)\s+/, ''))}</Text>
          </View>,
        );
      } else if (line.trim() === '') {
        endParagraph();
      } else {
        paragraph.push(line.trim());
      }
    }
    endParagraph();
    return blocks;
  };

  /** A section's body while its document loads, or the note for when it has none. */
  const body = (p_document: { text: string | null | undefined; failure: string | null }, p_none: string) =>
    p_document.failure ? (
      <Text style={styles.empty}>Could not be read: {p_document.failure}</Text>
    ) : p_document.text === undefined ? (
      <ActivityIndicator color={colors.accent} />
    ) : p_document.text ? (
      markdown(p_document.text)
    ) : (
      <Text style={styles.empty}>{p_none}</Text>
    );

  const adds = installed
    ? [
        ...installed.controls.map(control => `${control.root}: ${control.path}`),
        ...installed.workspaces.map(workspace => `${workspace.title} (workspace)`),
        ...installed.models.map(model => `${model.title} (AI model)`),
        ...installed.pipelines.map(pipeline => pipeline.title),
        ...installed.scripts.map(script => script.title),
      ]
    : [];
  const version_ = installed?.version ?? version;
  // The author the plugin names, or failing that the author part of its id.
  const author = installed?.author ?? listedAuthor ?? installed?.id.split('.')[0];

  return (
    <View style={styles.container}>
      <View style={[commonStyles.header, { alignItems: 'center', justifyContent: 'flex-start', gap: 8 }]}>
        <Pressable accessibilityRole="button" accessibilityLabel="Back to plugins" hitSlop={8} onPress={onBack} style={styles.back}>
          <ChevronLeft size={24} color={colors.accent} strokeWidth={1.8} />
        </Pressable>
        <Text numberOfLines={1} style={styles.title}>
          {name}
        </Text>
      </View>
      <ScrollView contentContainerStyle={styles.content}>
        {(author || version_) && (
          <Text style={styles.meta}>{[author && `by ${author}`, version_ && `version ${version_}`].filter(Boolean).join(' · ')}</Text>
        )}
        {(installed?.description || summary) && <Text style={styles.text}>{installed?.description || summary}</Text>}
        {installed && installed.tags.length > 0 && (
          <View style={styles.tags}>
            {installed.tags.map(tag => (
              <Text key={tag} style={styles.tag}>
                {tag}
              </Text>
            ))}
          </View>
        )}
        {installed && installed.screenshotPaths.length > 0 && (
          <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.shots} style={{ marginTop: 8 }}>
            {installed.screenshotPaths.map(file => (
              <Image key={file} source={{ uri: `file://${file}` }} style={styles.shot} resizeMode="cover" />
            ))}
          </ScrollView>
        )}

        <View style={styles.tabs}>
          {tabs.map(each => (
            <Pressable
              key={each}
              accessibilityRole="tab"
              accessibilityState={{ selected: tab === each }}
              onPress={() => setTab(each)}
              style={[styles.tab, tab === each && { borderBottomColor: colors.accent }]}
            >
              <Text style={[styles.tabText, tab === each && { color: colors.textPrimary }]}>{each}</Text>
            </Pressable>
          ))}
        </View>

        {tab === 'Overview' && body(overview, 'This plugin has no store page.')}
        {tab === 'Package Content' &&
          (!installed ? (
            <Text style={styles.empty}>Install the plugin to see what it adds.</Text>
          ) : adds.length === 0 ? (
            <Text style={styles.empty}>This plugin adds nothing.</Text>
          ) : (
            adds.map(item => (
              <View key={item} style={styles.bullet}>
                <Text style={styles.text}>•</Text>
                <Text style={[styles.text, { flex: 1 }]}>{item}</Text>
              </View>
            ))
          ))}
        {tab === 'Releases' && body(notes, 'This plugin has no release notes.')}
        {tab === 'Dependencies' &&
          (!installed ? (
            <Text style={styles.empty}>Install the plugin to see what it needs.</Text>
          ) : installed.dependencies.length === 0 ? (
            <Text style={styles.empty}>This plugin needs no other plugins.</Text>
          ) : (
            installed.dependencies.map(id => {
              const dependency = describe?.(id);
              return (
                <View key={id} style={{ gap: 2, marginBottom: 8 }}>
                  <Text style={[styles.text, { fontWeight: '600' }]}>
                    {dependency?.name ?? id}
                    {installed.missingDependencies.includes(id) ? ' (not installed)' : ''}
                  </Text>
                  {dependency?.name && <Text style={styles.meta}>{id}</Text>}
                  {dependency?.description ? <Text style={styles.text}>{dependency.description}</Text> : null}
                </View>
              );
            })
          ))}
      </ScrollView>
    </View>
  );
}
