//! Full join, panics if not found.

#[test]
fn vec_tuple_vec_tuple_into_value() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(must = root.values[..], as = vals, on(axis.0 = vals.0))]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[from_axis(axis.1)]
            direct_value: u8,
            #[select(select = vals.0)]
            joined_value_key: u32,
            #[select(select = vals.1)]
            joined_value: u16,
        }
    }

    let rows = Root {
        axis: vec![(1, 10), (2, 20), (3, 30)],
        values: vec![(1, 100), (2, 200), (3, 300)],
    }
    .to_rows()
    .axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].direct_value, 10);
    assert_eq!(rows[0].joined_value_key, 1);
    assert_eq!(rows[0].joined_value, 100);
    assert_eq!(rows[1].id, 2);
    assert_eq!(rows[1].direct_value, 20);
    assert_eq!(rows[1].joined_value_key, 2);
    assert_eq!(rows[1].joined_value, 200);
    assert_eq!(rows[2].id, 3);
    assert_eq!(rows[2].direct_value, 30);
    assert_eq!(rows[2].joined_value_key, 3);
    assert_eq!(rows[2].joined_value, 300);
}

#[test]
#[should_panic(expected = "rowview must join found no matching item")]
fn panics_when_item_not_found() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(must = root.values[..], as = vals, on(axis.0 = vals.0))]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[select(select = vals.1)]
            joined_value: u16,
        }
    }

    let _rows = Root {
        axis: vec![(1, 10), (2, 20)],
        values: vec![(1, 100)],
    }
    .to_rows()
    .axis_rows;
}
