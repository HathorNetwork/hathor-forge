// Fix wallet-lib bigIntReviver for engines where JSON.parse doesn't pass
// context.source to the reviver (WebKit < 18.4).
// Without context.source, BigInt(undefined) throws and the explorer breaks.
//
// Strategy: wrap JSON.parse so that when context.source is missing, we skip
// the BigInt conversion entirely and return numbers as-is. This means very
// large integers (>2^53) won't be BigInts on old engines, but the explorer
// won't crash — an acceptable tradeoff for local dev.
(function() {
  var hasContextSource = false;
  try {
    JSON.parse('1', function(_k, _v, ctx) {
      hasContextSource = !!(ctx && typeof ctx.source === 'string');
    });
  } catch(e) {}
  if (hasContextSource) return;

  var _parse = JSON.parse;
  JSON.parse = function(text, reviver) {
    if (typeof reviver !== 'function') {
      return _parse.call(JSON, text, reviver);
    }
    // Wrap the reviver to provide a synthetic context when missing
    return _parse.call(JSON, text, function(key, value) {
      // On engines without context.source, the reviver gets only (key, value).
      // We synthesize a context object. For numbers, use String(value) as source
      // which is imprecise for very large ints but prevents crashes.
      var context = { source: typeof value === 'number' ? String(value) : JSON.stringify(value) };
      try {
        return reviver.call(this, key, value, context);
      } catch(e) {
        // If the reviver still fails (e.g. BigInt on a float), return value as-is
        return value;
      }
    });
  };
})();
