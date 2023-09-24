/**
 * @param {String} base64
 */
const bytesFromBase64 = (base64, type = "application/octet-stream") =>
  fetch(`data:${type};base64,${base64}`)
    .then((res) => res.blob())
    .then((blob) => blob.arrayBuffer())
    .then((buf) => new Uint8Array(buf));

// is there a better way than this for the other direction
// b64 = require("js-base64")
// base64FromBytes = (bytes) => b64.fromUint8Array(bytes)
