const log = console.log.bind(console);

function toJs(instance, alwaysCopyData) {
  const view = new DataView(instance.exports.memory.buffer);
  const ptr = view.getUint32(instance.exports.JS, true);
  const len = view.getUint32(instance.exports.JS + 4, true);
  const code = new TextDecoder().decode(view.buffer.slice(ptr, ptr + len));
  return import(encodeURI("data:text/javascript," + code)).then((module) =>
    module.default(instance, alwaysCopyData)
  );
}

const rs = await WebAssembly.instantiateStreaming(
  fetch(`http://127.0.0.1:8000/to_js_testing.wasm`)
).then((results) => toJs(results.instance));

console.log(rs.add(2, 2));