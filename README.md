

# Features

- getters
  - fields
  - method
  - closure
- join
  - left
  - must
  - custom filler
- binding
 - copy - one element at a specific path copied into many records
 - multiple rowsets
- containers
  - vector as map (tuple first element and property)
  - map
- context
  - incrementers (+1, +2 for copy value; counters)
- correctness
   - invariants
   - bijection of mapping
   - fail fast or result error
- semantics
  - update-form aware types like `Option<Option<T>>`, `Option<T>`, `Either<T, ()>`, or `Either<T, T>`, where `T` may or may not be `Default`.
  - delta-aggregatable or final-value aware (last written is correct)

# Constraints and limitations

- Does not `Clone` input structures.
- Supports conversion of data from trusted sources only.
- Keys must be `Copy`.

# Guidelines and opinions

Macro is:
- token-based, not string-based
- Rust syntax in attributes for deep integration with the language and no need to learn new syntax
- compile-time typed
- reje
- Summable types has delta word in name of fields (unless we agreed at some point on types).

- Fail-fast panic is preferred to returning an error.
- avoids closures in attribues using closures because they are not declarative per se.
- forbits closusers and function to be mapped
- latest element in keyed, but not unique keyed collection, considered to be target of join by id

# Performance

Need to decide whether to build maps/sets for joins. I have not done it yet.
Need to read a join optimization article.
I guess then ask generate stress code and run under gungraun,
and ask optimize.