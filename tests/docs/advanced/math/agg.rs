//! Sum of keyed deltas and checked adds and final must panic or result.

use std::collections::HashMap;

#[test]
fn sum_over_left_join() {
    struct Root {
        a: Vec<(u64, ())>,
        b: Vec<(u64, u16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = abcs, axis = root.a)]
        #[joins(left = root.b[..], as = b, on = axis.0 == b.0)]
        struct Abcs {
            #[from_axis(axis.0)]
            a: u64,
            #[agg(sum = b.1)]
            b_sum: u32,
        }
    }

    let rows = Root {
        a: vec![(1, ()), (2, ()), (3, ())],
        b: vec![(1, 10), (1, 20), (3, 7)],
    }
    .to_rows()
    .abcs;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].a, 1);
    assert_eq!(rows[0].b_sum, 30);
    assert_eq!(rows[1].a, 2);
    assert_eq!(rows[1].b_sum, 0);
    assert_eq!(rows[2].a, 3);
    assert_eq!(rows[2].b_sum, 7);
}

#[test]
fn sum_over_hash_map_left_join() {
    struct Root {
        a: Vec<(u64, ())>,
        b: HashMap<u64, u16>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = abcs, axis = root.a)]
        #[joins(left = root.b, as = b, on = axis.0 == *b.0)]
        struct Abcs {
            #[from_axis(axis.0)]
            a: u64,
            #[agg(sum = *b.1)]
            b_sum: u32,
        }
    }

    let rows = Root {
        a: vec![(1, ()), (2, ()), (3, ())],
        b: HashMap::from([(1, 30), (3, 7)]),
    }
    .to_rows()
    .abcs;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].a, 1);
    assert_eq!(rows[0].b_sum, 30);
    assert_eq!(rows[1].a, 2);
    assert_eq!(rows[1].b_sum, 0);
    assert_eq!(rows[2].a, 3);
    assert_eq!(rows[2].b_sum, 7);
}
