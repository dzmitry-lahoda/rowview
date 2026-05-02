//! Allows duplicates for the same key in non-unique key collections.
//! In this case, uses the latest item for the join.

#[test]
fn left_join_uses_latest_duplicate_key() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(left = root.values[..], as = vals, on(axis.0 = vals.0))]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[select(select = vals.1)]
            joined_value: Option<u16>,
        }
    }

    let rows = Root {
        axis: vec![(1, 10), (2, 20), (3, 30)],
        values: vec![(1, 100), (2, 200), (1, 101), (3, 300), (2, 201)],
    }
    .to_rows()
    .axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows.id[0], 1);
    assert_eq!(rows.joined_value[0], Some(101));
    assert_eq!(rows.id[1], 2);
    assert_eq!(rows.joined_value[1], Some(201));
    assert_eq!(rows.id[2], 3);
    assert_eq!(rows.joined_value[2], Some(300));
}
