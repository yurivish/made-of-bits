// vite.config.js
import { resolve } from 'path';
import { defineConfig } from 'vite';
;
;

export default defineConfig({
  build: {
    lib: {
      formats: ['es'],
      // Could also be a dictionary or array of multiple entry points
      entry: resolve(__dirname, 'src/index.js'),
      name: 'Bit Structures',
      // the proper extensions will be added
      fileName: 'bit-structures',
    },
    rollupOptions: {
      // make sure to externalize deps that shouldn't be bundled
      // into your library
      // external: ['vue'],
      // output: {
      //   // Provide global variables to use in the UMD build
      //   // for externalized deps
      //   globals: {
      //     vue: 'Vue',
      //   },
      // },
    },
  },
});