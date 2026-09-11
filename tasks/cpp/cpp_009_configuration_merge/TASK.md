# CPP-009: JSON-like configuration merge

Complete the C++20 configuration merge implementation declared in
`include/config/value.hpp`, without changing its public API or adding external
dependencies. Build with CMake and run the visible CTest suite.

`hase::config::Value` is an in-memory JSON-like value. It supports null,
boolean, signed 64-bit integer, double, string, array, and object values. This
task is about merging already constructed values; parsing or serializing JSON
is outside its scope.

`merge(base, override_value)` returns a new value and must not modify either
input. The rules are exact:

* If both values are objects, merge their keys recursively. A key only in
  `base` is retained; a key only in `override_value` is copied to the result;
  a key in both is the result of merging those two values.
* Arrays are replaced as whole values. They are never concatenated or merged
  element-by-element.
* Every non-object override replaces the base value, including null, false,
  zero, and an empty string. An array override also replaces the base array as
  a whole value, including when it is empty.
* If the two values have different types, the override replaces the base. In
  particular, integer and double are distinct alternatives: merging an integer
  with a double produces that double, without numeric coercion.

An absent object key is not the same as a present key whose value is null.
Thus an override containing `{"setting": null}` leaves a present `setting`
key with a null value; it does not remove the base key.

The returned value must own an independent recursive copy. Mutating the result
through its non-const `as_array()` or `as_object()` accessors after `merge`
returns must not change either input. Conversely, later mutations to either
input must not change the returned result. The usual `Value` copy operations
must have the same value semantics.

The API deliberately exposes `std::variant` alternatives and checked
accessors. It does not require JSON text parsing, object-key ordering other
than the supplied `std::map` ordering, deletion markers, schema validation,
numeric coercion, special floating-point handling, or structural sharing.
