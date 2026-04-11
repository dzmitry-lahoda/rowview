//! Sum over nested values.

#[test]
fn sum_nested_values_into_axis_rows() {
    struct Root {
        a: Vec<(u64, B)>,
    }

    struct B {
        cs: Vec<u32>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = abcs, axis = root.a)]
        struct Abcs {
            #[agg(sum = axis.1.cs)]
            cs: u32,
        }
    }

    let rows = Root {
        a: vec![(1, B { cs: vec![10, 20] }), (2, B { cs: vec![3, 4, 5] })],
    }
    .to_rows()
    .abcs;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].cs, 30);
    assert_eq!(rows[1].cs, 12);
}

/// Same as simple sum, but with a lifted accumulator.
/// For u32, lift to u64 via into.
/// Can set own accumulators as needed.
#[test]
fn sum_nested_values_into_axis_rows_with_cast() {
    struct Root {
        a: Vec<(u64, B)>,
    }

    struct B {
        cs: Vec<u32>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = abcs, axis = root.a)]
        struct Abcs {
            #[agg(sum = axis.1.cs, convert = into)]
            cs: u64,
        }
    }

    let rows = Root {
        a: vec![
            (
                1,
                B {
                    cs: vec![u32::MAX, 1],
                },
            ),
            (2, B { cs: vec![3, 4, 5] }),
        ],
    }
    .to_rows()
    .abcs;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].cs, 4_294_967_296);
    assert_eq!(rows[1].cs, 12);
}
