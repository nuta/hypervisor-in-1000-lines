import { defineConfig } from 'vitepress'
import {
  groupIconMdPlugin,
  groupIconVitePlugin,
} from 'vitepress-plugin-group-icons'


// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "Hypervisor in 1,000 Lines",
  description: "Write your first hypervisor from scratch, in 1K LoC.",
  cleanUrls: true,
  markdown: {
    config(md) {
      md.use(groupIconMdPlugin)
    },
  },
  vite: {
    plugins: [
      groupIconVitePlugin()
    ],
  },
  locales: {
    en: {
      label: 'English',
      lang: 'en',
      themeConfig: {
        sidebar: [
          {
            text: 'Table of Contents',
            items: [
              { link: '/en/', text: '00. Intro' },
              { link: '/en/01-getting-started', text: '01. Getting Started' },
              { link: '/en/02-boot', text: '02. Boot' },
              { link: '/en/03-hello-world', text: '03. Hello World' },
              { link: '/en/04-memory-allocation', text: '04. Memory Allocation' },
              { link: '/en/05-guest-mode', text: '05. Guest Mode' },
              { link: '/en/06-guest-page-table', text: '06. Guest Page Table' },
              { link: '/en/07-hello-from-guest', text: '07. Hello from Guest' },
              { link: '/en/08-build-linux-kernel', text: '08. Build Linux Kernel' },
              { link: '/en/09-boot-linux', text: '09. Boot Linux' },
              { link: '/en/10-supervisor-binary-interface', text: '10. Supervisor Binary Interface' },
              { link: '/en/11-memory-mapped-io', text: '11. Memory-Mapped I/O' },
              { link: '/en/12-virtio', text: '12. Virtio' },
              { link: '/en/13-virtio-blk', text: '13. Virtio-blk' },
            ]
          },
          {
            text: 'Links',
            items: [
              { link: 'https://github.com/nuta/hypervisor-in-1000-lines', text: 'GitHub repository' },
            ]
          },
        ],
        socialLinks: [
          { icon: 'github', link: 'https://github.com/nuta/hypervisor-in-1000-lines' }
        ]
      }
    },
  },
})
