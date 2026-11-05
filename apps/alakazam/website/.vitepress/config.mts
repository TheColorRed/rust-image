import { defineConfig } from 'vitepress';

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: 'Alakazam',
  description: 'Abra. Kadabra. Alakazam!',
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Install', link: '/install' },
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Plugins', link: '/plugins/' },
    ],

    sidebar: {
      '/plugins/': [
        {
          text: 'Plugins',
          items: [
            { text: 'Overview', link: '/plugins/' },
            { text: 'Beauty', link: '/plugins/beauty' },
            { text: 'Object Remover', link: '/plugins/object-remover' },
            { text: 'Collage', link: '/plugins/collage' },
          ],
        },
      ],
      '/': [
      {
        text: 'Guide',
        items: [
          { text: 'Getting started', link: '/guide/getting-started' },
          { text: 'Fix and adjust a photo', link: '/guide/adjust' },
          { text: 'Clean up a photo', link: '/guide/cleanup' },
          { text: 'Give a photo a look', link: '/guide/looks' },
          { text: 'Portraits and faces', link: '/guide/portraits' },
          { text: 'Smart features', link: '/guide/smart-features' },
          { text: 'Plugins', link: '/guide/plugins' },
          { text: 'Develop a plugin', link: '/guide/plugin-development' },
          { text: 'Projects', link: '/guide/projects' },
          { text: 'Settings', link: '/guide/settings' },
        ],
      },
      ],
    },

    // socialLinks: [
    // { icon: 'github', link: 'https://github.com/vuejs/vitepress' }
    // ],
  },
});
