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
  minify: false,
  format: 'esm',
  define: { 
    // This should be defined to true or false, depending on whether we want to turn
    // on debug checks. These will slow down the code but help to catch implementation
    // bugs and invariant violations.
    // DEBUG: 'false',
    // globalThis: "false",
    // In order for code that uses the global debug flag to work correctly, we define
    // the attribute on `globalThis` so that `DEBUG && ...` statements resolve properly
    // during testing. In production, the debug variable will be replaced, so we need not
    // reference globalThis.
    // Hence, we define this replacement in order that our module not mutate global state.
    // This is all a bit ugly, but it works until I figure out a better way.

  },
  plugins: [examplePlugin],
  outfile: 'dist/made-of-bits.js', 
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
