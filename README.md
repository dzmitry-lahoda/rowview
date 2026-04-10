

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
 - copy - one element at specific path copued into many records
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
  - update forms aware like `Option<Option<T>>` or `Option<T>` or `Either<T, ()>`  or `Either<T,T>` , where T may or may be Default. 
  - delta(aggeregatable) or final value aware (last written is correct)

# Constraaints and limitations

- Does not `Clone` input structures.
- Supports conversion of data from trusted source only.
- Keys must be `Copy`

# Guidelines and opinionations

Macro is:
- token based, not string based
- Rust syntax in attributed for deep integration with language and no need to lear new syntax
- compile time typed

Fails fast panic is preffered to return error.