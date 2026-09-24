import { utimesSync } from 'fs';
import { defineConfig } from 'vitepress';
import { withSidebar } from 'vitepress-sidebar';

// Works around vitepress-sidebar's HMR plugin only reloading the sidebar on file
// add/unlink, not on frontmatter edits to an existing file (its 'change' handler
// never checks for .md files).
function sidebarFrontmatterReloadPlugin() {
  return {
    name: 'sidebar-frontmatter-reload',
    apply: 'serve' as const,
    configureServer(server: any) {
      server.watcher.on('change', (file: string) => {
        if (!file.endsWith('.md')) return;
        const configPath = (globalThis as any).VITEPRESS_CONFIG?.configPath;
        if (!configPath) return;
        const now = new Date();
        try {
          utimesSync(configPath, now, now);
        } catch {
          // config path may be stale during startup/shutdown
        }
      });
    }
  };
}

const baseConfig = defineConfig({
  title: "Abra",
  description: "Abra image library",
  srcDir: "./public",
  vite: {
    plugins: [sidebarFrontmatterReloadPlugin()],
    server: {
      watch: {
        ignored: ['!**/public/**']
      }
    }
  },
  themeConfig: {
    nav: [
      { text: 'Docs', link: '/getting-started' },
    ],
  }
});


// https://vitepress.dev/reference/site-config
export default withSidebar(baseConfig, {
  documentRootPath: './public',
  useTitleFromFrontmatter: true,
  includeRootIndexFile: false,
  includeFolderIndexFile: true,
  sortMenusByFrontmatterOrder: true,
  collapsed: true,
  hyphenToSpace: true,
  capitalizeEachWords: true
});
