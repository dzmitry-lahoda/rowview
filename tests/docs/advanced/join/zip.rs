//! Keyed zip join. Each axis key must exist in the joined collection, and each joined key must
//! exist in the axis collection.

#[test]
fn zip_success() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(zip = root.values[..], as = vals, on(axis.0 = vals.0))]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[from_axis(axis.1)]
            direct_value: u8,
            #[select(select = vals.1)]
            joined_value: u16,
        }
    }

    let rows = Root {
        axis: vec![(1, 10), (2, 20)],
        values: vec![(2, 200), (1, 100)],
    }
    .to_rows()
    .axis_rows;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows.id[0], 1);
    assert_eq!(rows.direct_value[0], 10);
    assert_eq!(rows.joined_value[0], 100);
    assert_eq!(rows.id[1], 2);
    assert_eq!(rows.direct_value[1], 20);
    assert_eq!(rows.joined_value[1], 200);
}

#[test]
#[should_panic(expected = "rowview must join found no matching item")]
fn zip_panics_when_axis_item_has_no_joined_item() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(zip = root.values[..], as = vals, on(axis.0 = vals.0))]
        struct AxisRow {
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

#[test]
#[should_panic(expected = "rowview zip join found joined item with no matching axis item")]
fn zip_panics_when_joined_item_has_no_axis_item() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(zip = root.values[..], as = vals, on(axis.0 = vals.0))]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[select(select = vals.1)]
            joined_value: u16,
        }
    }

    let _rows = Root {
        axis: vec![(1, 10)],
        values: vec![(1, 100), (2, 200)],
    }
    .to_rows()
    .axis_rows;
}
