//! Primitive values, simple structs, single depth, single item, single row, and single arrays.
//! One feature at a time.

use subdef::subdef;

#[test]
fn singleton() {
    #[subdef]
    struct Singleton {
        b: u32,
    }

    #[rowview::rows(root = Singleton)]
    mod schema {
        #[rowset(name = abs, axis = ())]
        struct Ab {
            #[copy(root.b)]
            ab: u32,
        }
    }

    let singleton = Singleton { b: 42 };
    let rows = singleton.to_rows().abs;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.ab[0], 42);
}

#[test]
fn deep_singleton() {
    #[subdef]
    struct Singleton {
        b: u32,
        c: [_; {
            struct C {
                d: [_; {
                    struct D {
                        e: u8,
                    }
                }],
            }
        }],
    }

    #[rowview::rows(root = Singleton)]
    mod schema {
        #[rowset(name = abs, axis = ())]
        struct Ab {
            #[copy(root.b)]
            ab: u32,
            #[copy(root.c.d.e)]
            cde: u8,
        }
    }

    let singleton = Singleton {
        b: 42,
        c: C { d: D { e: 33 } },
    };
    let rows = singleton.to_rows().abs;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.ab[0], 42);
    assert_eq!(rows.cde[0], 33);
}
