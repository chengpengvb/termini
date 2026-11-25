globalThis.window = globalThis;
globalThis.self = globalThis;
globalThis.global = globalThis;

globalThis.console = {
  log: function () { __print(Array.prototype.map.call(arguments, String).join(' ')); },
  info: function () { __print(Array.prototype.map.call(arguments, String).join(' ')); },
  warn: function () { __print('WARN ' + Array.prototype.map.call(arguments, String).join(' ')); },
  error: function () { __print('ERROR ' + Array.prototype.map.call(arguments, String).join(' ')); },
  debug: function () {}
};

globalThis.__timers = [];
globalThis.setTimeout = function (fn) { globalThis.__timers.push(fn); return globalThis.__timers.length; };
globalThis.setInterval = function () { return 0; };
globalThis.clearTimeout = function () {};
globalThis.clearInterval = function () {};
globalThis.setImmediate = function (fn) { return globalThis.setTimeout(fn, 0); };
globalThis.__drainTimers = function () {
  var pending = globalThis.__timers;
  globalThis.__timers = [];
  for (var i = 0; i < pending.length; i++) { pending[i](); }
  return pending.length;
};

function TextEncoder() {}
TextEncoder.prototype.encode = function (input) {
  var str = String(input === undefined ? '' : input);
  var bytes = [];
  for (var i = 0; i < str.length; i++) {
    var code = str.charCodeAt(i);
    if (code >= 0xd800 && code <= 0xdbff && i + 1 < str.length) {
      var next = str.charCodeAt(i + 1);
      if (next >= 0xdc00 && next <= 0xdfff) {
        code = 0x10000 + ((code - 0xd800) << 10) + (next - 0xdc00);
        i++;
      }
    }
    if (code < 0x80) {
      bytes.push(code);
    } else if (code < 0x800) {
      bytes.push(0xc0 | (code >> 6), 0x80 | (code & 0x3f));
    } else if (code < 0x10000) {
      bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
    } else {
      bytes.push(0xf0 | (code >> 18), 0x80 | ((code >> 12) & 0x3f), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
    }
  }
  return new Uint8Array(bytes);
};

function TextDecoder() {}
TextDecoder.prototype.decode = function (input) {
  if (input === undefined) { return ''; }
  var bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  var out = '';
  var i = 0;
  while (i < bytes.length) {
    var byte = bytes[i++];
    var code;
    if (byte < 0x80) {
      code = byte;
    } else if (byte >= 0xc0 && byte < 0xe0) {
      code = ((byte & 0x1f) << 6) | (bytes[i++] & 0x3f);
    } else if (byte >= 0xe0 && byte < 0xf0) {
      code = ((byte & 0x0f) << 12) | ((bytes[i++] & 0x3f) << 6) | (bytes[i++] & 0x3f);
    } else {
      code = ((byte & 0x07) << 18) | ((bytes[i++] & 0x3f) << 12) | ((bytes[i++] & 0x3f) << 6) | (bytes[i++] & 0x3f);
    }
    if (code > 0xffff) {
      code -= 0x10000;
      out += String.fromCharCode(0xd800 + (code >> 10), 0xdc00 + (code & 0x3ff));
    } else {
      out += String.fromCharCode(code);
    }
  }
  return out;
};

globalThis.TextEncoder = TextEncoder;
globalThis.TextDecoder = TextDecoder;

var B64_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

function bytesToBase64(bytes) {
  var out = '';
  var i = 0;
  for (; i + 2 < bytes.length; i += 3) {
    var n = (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];
    out += B64_CHARS[(n >> 18) & 63] + B64_CHARS[(n >> 12) & 63] + B64_CHARS[(n >> 6) & 63] + B64_CHARS[n & 63];
  }
  if (i + 1 === bytes.length) {
    var a = bytes[i] << 16;
    out += B64_CHARS[(a >> 18) & 63] + B64_CHARS[(a >> 12) & 63] + '==';
  } else if (i + 2 === bytes.length) {
    var b = (bytes[i] << 16) | (bytes[i + 1] << 8);
    out += B64_CHARS[(b >> 18) & 63] + B64_CHARS[(b >> 12) & 63] + B64_CHARS[(b >> 6) & 63] + '=';
  }
  return out;
}

function base64ToBytes(text) {
  var clean = String(text).replace(/[^A-Za-z0-9+/]/g, '');
  var out = [];
  for (var i = 0; i < clean.length; i += 4) {
    var n = (B64_CHARS.indexOf(clean[i]) << 18) | (B64_CHARS.indexOf(clean[i + 1]) << 12) | ((B64_CHARS.indexOf(clean[i + 2]) & 63) << 6) | (B64_CHARS.indexOf(clean[i + 3]) & 63);
    out.push((n >> 16) & 255);
    if (i + 2 < clean.length) { out.push((n >> 8) & 255); }
    if (i + 3 < clean.length) { out.push(n & 255); }
  }
  return new Uint8Array(out);
}

globalThis.atob = function (text) { return new TextDecoder().decode(base64ToBytes(text)); };
globalThis.btoa = function (text) { return bytesToBase64(new TextEncoder().encode(text)); };

function toBytes(data) {
  if (data instanceof Uint8Array) { return data; }
  if (data instanceof ArrayBuffer) { return new Uint8Array(data); }
  if (data && data.buffer instanceof ArrayBuffer) { return new Uint8Array(data.buffer); }
  if (!(data instanceof Object)) { return new TextEncoder().encode(data); }
  throw new Error('unsupported data type for workspace.write');
}

globalThis.__describeError = function (error) {
  if (!error) { return 'unknown error'; }
  if (!(error instanceof Object)) { return String(error); }
  var head = (error.name ? error.name + ': ' : '') + (error.message !== undefined ? error.message : String(error));
  return error.stack ? head + '\n' + error.stack : head;
};

globalThis.saveDocx = function (path, doc) {
  return docx.Packer.toBase64String(doc).then(function (base64) { return workspace.writeBase64(path, base64); });
};

globalThis.savePptx = function (path, pptx) {
  return pptx.write({ outputType: 'base64' }).then(function (base64) { return workspace.writeBase64(path, base64); });
};

function hostCall(value) {
  if (!(value instanceof Object) && String(value).indexOf('\u0000ERR:') === 0) { throw new Error(String(value).slice(5)); }
  return value;
}

globalThis.workspace = {
  write: function (path, data) { return hostCall(__write_file(String(path), bytesToBase64(toBytes(data)))); },
  writeBase64: function (path, base64) { return hostCall(__write_file(String(path), String(base64))); },
  read: function (path) { return base64ToBytes(hostCall(__read_file(String(path)))); },
  readText: function (path) { return new TextDecoder().decode(base64ToBytes(hostCall(__read_file(String(path))))); }
};
