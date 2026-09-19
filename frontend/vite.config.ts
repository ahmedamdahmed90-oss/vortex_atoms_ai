import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/v1': { target: 'http://localhost:8080', changeOrigin: true },
      '/ws': { target: 'ws://localhost:8080', ws: true },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          if (id.includes('node_modules')) {
            // NOTE: check syntax-highlighter before the generic 'react' rule:
            // 'react-syntax-highlighter' contains the substring 'react' and would
            // otherwise be forced into 'vendor'. Styles stay in vendor (tiny,
            // statically imported); the Prism engine ships as a lazy chunk.
            if (id.includes('react-syntax-highlighter') && !id.includes('/styles/')) return 'syntax-highlighter';
            if (id.includes('react') || id.includes('react-dom') || id.includes('react-router')) return 'vendor';
            if (id.includes('lucide-react') || id.includes('clsx') || id.includes('tailwind-merge')) return 'ui';
            if (id.includes('zustand') || id.includes('@tanstack/react-query') || id.includes('react-markdown')) return 'utils';
            return 'vendor';
          }
        },
      },
    },
  },
})