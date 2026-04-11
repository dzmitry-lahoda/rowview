//! Allows joining by key.

#[test]
fn vec_tuple_vec_tuple_into_option() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(left = root.values[..], as = vals, on = axis.0 == vals.0)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[from_axis(axis.1)]
            direct_value: u8,
            #[select(select = vals.0)]
            joined_value_key: Option<u32>,
            #[select(select = vals.1)]
            joined_value: Option<u16>,
        }
    }

    let rows = Root {
        axis: vec![(1, 10), (2, 20), (3, 30)],
        values: vec![(2, 200), (3, 300), (4, 400)],
    }
    .to_rows()
    .axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].direct_value, 10);
    assert_eq!(rows[0].joined_value_key, None);
    assert_eq!(rows[0].joined_value, None);
    assert_eq!(rows[1].id, 2);
    assert_eq!(rows[1].direct_value, 20);
    assert_eq!(rows[1].joined_value_key, Some(2));
    assert_eq!(rows[1].joined_value, Some(200));
    assert_eq!(rows[2].id, 3);
    assert_eq!(rows[2].direct_value, 30);
    assert_eq!(rows[2].joined_value_key, Some(3));
    assert_eq!(rows[2].joined_value, Some(300));
}

#[test]
fn vec_tuple_array_into_option() {}

#[test]
fn multi() {}

#[test]
fn std_map() {}
