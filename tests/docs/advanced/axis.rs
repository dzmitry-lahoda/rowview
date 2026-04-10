//! Double depth axis, custom collections.

use std::collections::{HashMap, hash_map};

struct DirtyMap {
    inner: HashMap<u32, u32>,
}

impl DirtyMap {
    fn from_entries(entries: [(u32, u32); 3]) -> Self {
        Self {
            inner: HashMap::from(entries),
        }
    }

    fn iter(&self) -> hash_map::Iter<'_, u32, u32> {
        self.inner.iter()
    }
}

#[test]
fn custom_map_wrapper_can_be_axis() {
    struct Root {
        axis: DirtyMap,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        struct AxisRow {
            #[from_axis(*axis.0)]
            id: u32,
            #[from_axis(*axis.1)]
            value: u32,
        }
    }

    let root = Root {
        axis: DirtyMap::from_entries([(1, 10), (3, 30), (2, 20)]),
    };

    let rows = root.to_rows().axis_rows;
    let mut pairs: Vec<_> = rows.into_iter().map(|row| (row.id, row.value)).collect();
    pairs.sort_unstable();

    assert_eq!(pairs, vec![(1, 10), (2, 20), (3, 30)]);
}


#[test]
fn nested_axis() {
    struct B {
        d: f32,
    }
    struct A {
        pub b: Vec<(u8, B)>,
    }
    struct Root {
        pub a: Vec<A>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.a[..].b)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u8,
            #[from_axis(axis.1.d)]
            ad: f32,
        }
    }

    let root = Root {
        a: vec![
            A {
                b: vec![(1, B { d: 10.0 }), (2, B { d: 20.0 })],
            },
            A {
                b: vec![(3, B { d: 30.0 })],
            },
        ],
    };

    let rows = root.to_rows().axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].ad, 10.0);
    assert_eq!(rows[1].id, 2);
    assert_eq!(rows[1].ad, 20.0);
    assert_eq!(rows[2].id, 3);
    assert_eq!(rows[2].ad, 30.0);
}


#[test]
fn nested_with_parent_copies() {
    struct B {
        d: f32,
    }
    struct A {
        pub parent_value: u128,
        pub b: Vec<(u8, B)>,
    }
    struct Root {
        pub root_value: i128,
        pub a: Vec<A>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.a[..].b)]
        struct AxisRow {
            #[copy(root.root_value)]
            root_value: i128,
            #[copy(root.a[..].parent_value)]
            parent_value: u128,

            #[from_axis(axis.0)]
            id: u8,
            #[from_axis(axis.1.d)]
            ad: f32,
        }
    }

    let root = Root {
        root_value: -7,
        a: vec![
            A {
                parent_value: 100,
                b: vec![(1, B { d: 10.0 }), (2, B { d: 20.0 })],
            },
            A {
                parent_value: 200,
                b: vec![(3, B { d: 30.0 })],
            },
        ],
    };

    let rows = root.to_rows().axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].root_value, -7);
    assert_eq!(rows[0].parent_value, 100);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].ad, 10.0);
    assert_eq!(rows[1].root_value, -7);
    assert_eq!(rows[1].parent_value, 100);
    assert_eq!(rows[1].id, 2);
    assert_eq!(rows[1].ad, 20.0);
    assert_eq!(rows[2].root_value, -7);
    assert_eq!(rows[2].parent_value, 200);
    assert_eq!(rows[2].id, 3);
    assert_eq!(rows[2].ad, 30.0);
}



// Case 2:
// #[copy(root.c)]

// Case 3:
// #[copy(root.a[..].d)]
// ad: f32
