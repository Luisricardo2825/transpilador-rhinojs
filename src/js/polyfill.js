this.Java = this.Java ?? {
  type: (cls) => importClass(cls),
  from: (obj) => objectFrom(obj),
};

function importClass(cls) {
  return cls.split(".").reduce((obj, part) => obj[part], Packages);
}
function objectFrom(obj) {
  if (obj == null) return obj;

  try {
    const JavaMap = Java.type("java.util.Map");
    const JavaList = Java.type("java.util.List");
    const JavaArray = Java.type("java.lang.reflect.Array");

    if (obj instanceof JavaMap) {
      const result = {};
      const it = obj.entrySet().iterator();
      while (it.hasNext()) {
        const e = it.next();
        result[e.getKey()] = objectFrom(e.getValue());
      }
      return result;
    }

    if (obj instanceof JavaList) {
      const arr = [];
      for (let i = 0; i < obj.size(); i++) arr.push(objectFrom(obj.get(i)));
      return arr;
    }

    if (obj.getClass && obj.getClass().isArray()) {
      const len = JavaArray.getLength(obj);
      const arr = [];
      for (let i = 0; i < len; i++) arr.push(objectFrom(JavaArray.get(obj, i)));
      return arr;
    }
  } catch (e) {
    // Silenciar erro para compatibilidade
  }

  return obj;
}
this.Java = Java;

(function (global) {
  // Inicializa ou reutiliza Java

  // Função genérica para criar polyfills
  const _polyfill = (obj, key, fn) => {
    if (obj !== undefined && typeof obj[key] === "undefined") Object.defineProperty(obj, key, { value: fn, configurable: true, writable: true, enumerable: true });
  };

  // ===== Função objectFrom =====

  // ===== Symbol =====
  if (typeof global.Symbol !== "function") {
    let id = 0;
    const registry = {};
    const Symbol = (description) => {
      if (new.target) throw new TypeError("Symbol is not a constructor");
      const tag = `@@Symbol(${description ?? ""}):${id++}`;
      return { __rhinoSymbol: true, toString: () => tag };
    };
    _polyfill(Symbol, "for", (key) => ((registry[key] == null ? Object.defineProperty(registry, key, { value: Symbol(key), configurable: true, writable: true, enumerable: true }).value : registry[key])));
    _polyfill(Symbol, "keyFor", (sym) => {
      for (let k in registry) if (registry[k] === sym) return k;
    });
    Symbol.iterator = Symbol("iterator");
    Symbol.hasInstance = Symbol("hasInstance");
    Symbol.set = (obj, key, value) => {
      if (key && key.__rhinoSymbol) {
        Object.defineProperty(obj, key.toString(), { value, configurable: true, writable: true, enumerable: false });
      } else Object.defineProperty(obj, key, { value, configurable: true, writable: true, enumerable: true });
      return value;
    };
    global.Symbol = Symbol;
  }

  // ===== Iteradores =====
  const makeIterator = (arr) => {
    let i = 0;
    return {
      next: () =>
        i < arr.length ? { value: arr[i++], done: false } : { done: true },
    };
  };
  _polyfill(Array.prototype, Symbol.iterator, function () {
    return makeIterator(this);
  });

  ["java.util.List", "java.util.Map", "java.util.Set"].forEach((cls) => {
    try {
      const JC = Java.type(cls);
      if (JC && !JC.prototype[Symbol.iterator]) {
        _polyfill(JC.prototype, Symbol.iterator, function () {
          const it =
            cls === "java.util.Map"
              ? this.entrySet().iterator()
              : this.iterator();
          return {
            next: () => {
              if (!it.hasNext()) return { done: true };
              if (cls === "java.util.Map") {
                const e = it.next();
                return {
                  value: { key: e.getKey(), value: e.getValue() },
                  done: false,
                };
              }
              return { value: it.next(), done: false };
            },
          };
        });
      }
    } catch {}
  });

  // ===== Object polyfills =====
  _polyfill(Object, "isJavaObject", (obj) => {
    return typeof obj === "object" && obj != null && obj.getClass !== undefined;
  });

  _polyfill(Object, "is", (a, b) => {
    if (a === b) return a !== 0 || 1 / a === 1 / b;
    return a !== a && b !== b;
  });
  
  _polyfill(Object, "equals", (a, b) => {
    // Check if they are java objects if so, do java equals() method
    if (Object.isJavaObject(a)) return a.equals(b);

    if (Object.isJavaObject(b)) return false;
    
    return Object.is(a, b);
  });

  _polyfill(Object, "from", objectFrom);

  _polyfill(Object, "keys", (obj) => {
    const keys = [];
    for (let k in obj)
      if (Object.prototype.hasOwnProperty.call(obj, k)) keys.push(k);
    return keys;
  });

  _polyfill(Object, "values", (obj) => {
    const vals = [];
    for (let k in obj)
      if (Object.prototype.hasOwnProperty.call(obj, k)) vals.push(obj[k]);
    return vals;
  });

  _polyfill(Object, "entries", (obj) => {
    const entries = [];
    for (let k in obj)
      if (Object.prototype.hasOwnProperty.call(obj, k))
        entries.push([k, obj[k]]);
    return entries;
  });

  _polyfill(Object, "assign", function (target, ...sources) {
    if (target == null)
      throw new TypeError("Cannot convert undefined or null to object");
    for (let i = 1; i < arguments.length; i++) {
      const src = arguments[i];
      if (src != null) {
        for (let k in src)
          if (Object.prototype.hasOwnProperty.call(src, k)) target[k] = src[k];
      }
    }
    return target;
  });

  // ===== Array polyfills =====
  _polyfill(Array.prototype, "flat", function (depth = 1) {
    const result = [];
    const flatten = (arr, d) => {
      for (let i = 0; i < arr.length; i++) {
        const el = arr[i];
        if (Array.isArray(el) && d > 0) flatten(el, d - 1);
        else result.push(el);
      }
    };
    flatten(this, depth);
    return result;
  });

  _polyfill(Array.prototype, "flatMap", function (fn) {
    const result = [];
    for (let i = 0; i < this.length; i++) {
      const mapped = fn(this[i], i, this);
      if (Array.isArray(mapped)) {
        for (let j = 0; j < mapped.length; j++) result.push(mapped[j]);
      } else result.push(mapped);
    }
    return result;
  });

  _polyfill(Array.prototype, "find", function (fn) {
    for (let i = 0; i < this.length; i++)
      if (fn(this[i], i, this)) return this[i];
    return undefined;
  });

  _polyfill(Array.prototype, "findIndex", function (fn) {
    for (let i = 0; i < this.length; i++) if (fn(this[i], i, this)) return i;
    return -1;
  });

  _polyfill(Array.prototype, "includes", function (val, fromIndex) {
    const length = this.length >>> 0;
    let index = fromIndex == null ? 0 : fromIndex | 0;
    if (index < 0) index = Math.max(length + index, 0);
    for (; index < length; index++) {
      const current = this[index];
      if (current === val || (current !== current && val !== val)) return true;
    }
    return false;
  });

  _polyfill(Array.prototype, "some", function (fn) {
    for (let i = 0; i < this.length; i++) if (fn(this[i], i, this)) return true;
    return false;
  });

  _polyfill(Array.prototype, "every", function (fn) {
    for (let i = 0; i < this.length; i++)
      if (!fn(this[i], i, this)) return false;
    return true;
  });

  _polyfill(
    Array.prototype,
    "fill",
    function (value, start = 0, end = this.length) {
      for (let i = start; i < end; i++) this[i] = value;
      return this;
    },
  );

  _polyfill(
    Array.prototype,
    "copyWithin",
    function (target, start, end = this.length) {
      const len = this.length;
      target = target | 0;
      start = start | 0;
      end = end | 0;
      if (target < 0) target = Math.max(len + target, 0);
      if (start < 0) start = Math.max(len + start, 0);
      if (end < 0) end = Math.max(len + end, 0);
      const count = Math.min(end - start, len - target);
      if (start < target && target < start + count) {
        for (let i = count - 1; i >= 0; i--) this[target + i] = this[start + i];
      } else {
        for (let i = 0; i < count; i++) this[target + i] = this[start + i];
      }
      return this;
    },
  );

  // ===== Array estático =====
  _polyfill(Array, "from", function (obj) {
    if (obj == null) return [];
    const result = [];
    const length = obj.length;
    if (typeof length === "number") {
      for (let i = 0; i < length; i++) result.push(obj[i]);
      return result;
    }
    if (obj[Symbol.iterator]) {
      const iterator = obj[Symbol.iterator]();
      for (let item = iterator.next(); !item.done; item = iterator.next())
        result.push(item.value);
    }
    return result;
  });

  _polyfill(Array, "of", function () {
    const arr = [];
    for (let i = 0; i < arguments.length; i++) arr.push(arguments[i]);
    return arr;
  });
})(this);

Symbol.assign = function (obj, key, operator, value) {
  var current = obj[key];
  if (operator === '&&=' && !current) return current;
  if (operator === '||=' && current) return current;
  if (operator === '??=' && current != null) return current;
  value = typeof value === 'function' ? value() : value;
  switch (operator) {
    case '+=': return Symbol.set(obj, key, current + value);
    case '-=': return Symbol.set(obj, key, current - value);
    case '*=': return Symbol.set(obj, key, current * value);
    case '/=': return Symbol.set(obj, key, current / value);
    case '%=': return Symbol.set(obj, key, current % value);
    case '**=': return Symbol.set(obj, key, Math.pow(current, value));
    case '<<=': return Symbol.set(obj, key, current << value);
    case '>>=': return Symbol.set(obj, key, current >> value);
    case '>>>=': return Symbol.set(obj, key, current >>> value);
    case '|=': return Symbol.set(obj, key, current | value);
    case '^=': return Symbol.set(obj, key, current ^ value);
    case '&=': return Symbol.set(obj, key, current & value);
  }
  throw new TypeError('Unsupported Symbol assignment operator: ' + operator);
};

Symbol.instanceOf = function (value, constructor) {
  var handler = constructor && constructor[Symbol.hasInstance];
  if (typeof handler === "function") return !!handler.call(constructor, value);
  return !!(constructor && constructor.prototype && constructor.prototype.isPrototypeOf(value));
};