import { defineConfig, Plugin } from 'vite';
import { resolve } from 'path';
import fs from 'fs';

function extensionStaticPlugin(): Plugin {
  return {
    name: 'extension-static-plugin',
    closeBundle() {
      const distDir = resolve(__dirname, 'dist');
      if (!fs.existsSync(distDir)) {
        fs.mkdirSync(distDir, { recursive: true });
      }

      // 1. Move dist/src/popup.html to dist/popup.html with corrected asset paths
      const srcPopup = resolve(distDir, 'src/popup.html');
      const destPopup = resolve(distDir, 'popup.html');
      if (fs.existsSync(srcPopup)) {
        let html = fs.readFileSync(srcPopup, 'utf-8');
        // Fix relative references from ../assets/ to ./assets/
        html = html
          .replace(/(?:\.\.\/)+assets\//g, './assets/')
          .replace(/"\/assets\//g, '"./assets/')
          .replace(/'\/assets\//g, "'./assets/");
        fs.writeFileSync(destPopup, html, 'utf-8');
      }

      // 2. Copy manifest.json
      const manifestSrc = resolve(__dirname, 'manifest.json');
      const manifestDist = resolve(distDir, 'manifest.json');
      if (fs.existsSync(manifestSrc)) {
        fs.copyFileSync(manifestSrc, manifestDist);
      }

      // 3. Copy icons directory
      const iconsSrc = resolve(__dirname, 'icons');
      const iconsDist = resolve(distDir, 'icons');
      if (fs.existsSync(iconsSrc)) {
        fs.cpSync(iconsSrc, iconsDist, { recursive: true });
      }

      // 4. Clean up dist/src directory
      const distSrc = resolve(distDir, 'src');
      if (fs.existsSync(distSrc)) {
        try {
          fs.rmSync(distSrc, { recursive: true, force: true });
        } catch {}
      }
    },
  };
}

export default defineConfig({
  base: './',
  plugins: [extensionStaticPlugin()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        popup: resolve(__dirname, 'src/popup.html'),
        background: resolve(__dirname, 'src/background.ts'),
        content: resolve(__dirname, 'src/content.ts'),
      },
      output: {
        entryFileNames: (chunkInfo) => {
          if (chunkInfo.name === 'background' || chunkInfo.name === 'content') {
            return '[name].js';
          }
          return 'assets/[name].js';
        },
        chunkFileNames: 'assets/[name].js',
        assetFileNames: 'assets/[name].[ext]',
      },
    },
  },
});
