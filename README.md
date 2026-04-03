

# Features

- fields
- get method getters
- closure getters
- join_left
  - custom filler
- copy - one element at specific path copued into many records
- delta(aggeregatable) or final value aware (last written is correct)
- vector as map (tuple first element and property)
- custom incrementers (+1, +2 for copy value; counters)
- invariant hookson row constuction
- update forms aware like `Option<Option<T>>` or `Option<T>` or `Either<T, ()>`  or `Either<T,T>` , where T may or may be Default. 

# Limitations

- Supports conversion of data from trusted source only.
- Keys must be `Copy`


# Guideliens

Macro is:
- token based, not string based
- Rust syntax in attributed for deep integration with language and no need to lear new syntax
- compile time typed