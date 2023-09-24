import * as esbuild from 'esbuild';

const plural = n => n === 1 ? '' : 's';

let examplePlugin = {
  name: 'example',
  setup(build) {
    build.onEnd(result => {
      console.log(`build ended with ${result.errors.length} error${plural(result.errors.length)}`);
    });
  },
};

const ctx = await esbuild.context({
  entryPoints: ['./src/index.js'],
  bundle: true,
  format: 'esm',
  define: { DEBUG: 'false' }, // for debug builds (should be 'false' for not)
  plugins: [examplePlugin],
  outfile: 'dist/bit-structures.js', 
  sourcemap: 'inline'
});

await ctx.watch();
console.log('watching...');

const { host, port } = await ctx.serve({
  servedir: 'dist', 
  port: 4242,
  // SSL certificates for local serving generated using the following command,
  // which was taken from the esbuild documentation: https://esbuild.github.io/api/#https
  //
  // openssl req -x509 -newkey rsa:4096 -keyout silk.key -out silk.cert -days 9999 -nodes -subj /CN=127.0.0.1
  keyfile: 'silk.key',
  certfile: 'silk.cert',
});

console.log(`serving on https://${host}:${port}`);
