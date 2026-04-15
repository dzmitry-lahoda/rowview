

# Features

- virtual row existence
  - `#[support(any(root.a[..].id, root.b[..].id))]` creates a row domain from the union of keys found in the listed sources.
  - `#[bind(left = root.a, as = a, by = a.id)]` resolves an optional binding for each support key.
  - `#[bind(left = root.c, as = c, by = *c.id, on(all(any(a, b), not(d))))]` resolves an optional binding by key plus a restricted presence formula over earlier bindings.
  - `#[from_key(key)]` projects the virtual support key into a row field.
- getters
  - fields
  - method
  - closure
- join
  - left
  - must
  - inner
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
- soa output

# Constraints and limitations

- Crates using `rowview::rows` must depend on `soa_derive = "0.14"` directly.
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

# Virtual row existence

Some rows are not naturally generated from a single physical axis. For example, a row may need to exist when either an `A` record or a `B` record exists, while `C` and `D` are optional projections. In database terms, the row domain is a derived support relation:

```text
support = keys(A) UNION keys(B)
a = optional A[key]
b = optional B[key]
d = optional D[key]
c = optional C[key] when all(any(a, b), not(d))
```

The macro supports that shape with `support` and `bind`:

```rust
#[rowview::rows(root = Root)]
mod schema {
    #[rowset(name = rows)]
    #[support(any(root.a[..].id, root.b[..].id))]
    #[bind(left = root.a, as = a, by = a.id)]
    #[bind(left = root.b, as = b, by = b.id)]
    #[bind(left = root.d, as = d, by = d.id)]
    #[bind(
        left = root.c,
        as = c,
        by = *c.id,
        on(all(any(a, b), not(d)))
    )]
    struct Row {
        #[from_key(key)]
        id: u32,
        #[select(select = a.value)]
        a_value: Option<&'static str>,
        #[select(select = b.value)]
        b_value: Option<&'static str>,
        #[select(select = c.value)]
        c_value: Option<&'static str>,
        #[select(select = d.value)]
        d_value: Option<&'static str>,
    }
}
```

`support(any(...))` is a row-existence formula. Each expression uses the collection-slice form `root.collection[..].field`, so the collection before `[..]` is iterated and the field after it becomes the support key. The generated support keys are deduplicated in first-seen order, so the example above emits one row for each key present in `root.a` or `root.b`.

`bind` is evaluated for each support key, in declaration order. `by = ...` is the only item/key match: the expression is evaluated against the current source item and compared to the current support key. It may dereference ids, for example `by = *c.id` or `by = *c.0`.

`on(...)` is not an arbitrary Rust predicate. It is a restricted dependency formula over earlier bindings only. A bare binding alias means present, `not(alias)` means missing, and `any(...)` / `all(...)` compose those terms. For example, `on(all(any(a, b), not(d)))` means bind `c` only when either `a` or `b` is present and `d` is missing. The formula cannot inspect joined item properties such as `c.allowed`, and it cannot introduce another key comparison; joins use ids only through `by = ...`.

`select` over a `bind` returns `Option<T>`. Missing bindings project as `None`; present bindings project as `Some(...)`. The current implementation preserves the existing latest-match behavior by using the last matching source item for a binding.

`#[joins(inner = ..., as = ..., on(axis = value))]` is the narrow row-level inner join form. It still uses the same latest-match lookup rule as `left` and `must`, but a missing joined item skips the whole axis row instead of projecting `None` or panicking. Because row skipping changes row existence, `inner` is accepted only on row-level `#[joins(...)]`, not on field-level `#[join(...)]`.

Parsing intentionally lowers surface syntax into a lower-level row-existence model before code generation. `axis = ...` and `support/bind` are parsed as user-facing syntax, then validation produces a `RowExistencePlan`. Code generation consumes that plan instead of reinterpreting the original attributes. That keeps future sugar isolated from the core row-building algorithm.

# Performance

Need to decide whether to build maps/sets for joins. I have not done it yet.
Need to read a join optimization article.
I guess then ask generate stress code and run under gungraun,
and ask optimize.
