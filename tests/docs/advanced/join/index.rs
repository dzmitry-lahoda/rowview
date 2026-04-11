//! Joins by array position, assuming both collections are ordered.
//! Inner zip only.

#[test]
fn index_success() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<u16>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(index = root.values[..], as = vals)]
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

    let root = Root {
        axis: vec![(1, 10), (2, 20)],
        values: vec![100, 200],
    };

    let rows = root.to_rows().axis_rows;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].direct_value, 10);
    assert_eq!(rows[0].joined_value_key, Some(1));
    assert_eq!(rows[0].joined_value, Some(100));
    assert_eq!(rows[1].id, 2);
    assert_eq!(rows[1].direct_value, 20);
    assert_eq!(rows[1].joined_value_key, Some(2));
    assert_eq!(rows[1].joined_value, Some(200));
}

#[test]
#[should_panic(
    expected = "rowview index join requires axis and joined collection lengths to match"
)]
fn index_panics_when_joined_len_is_shorter() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<u16>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(index = root.values[..], as = vals)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[select(select = vals.1)]
            joined_value: Option<u16>,
        }
    }

    let root = Root {
        axis: vec![(1, 10), (2, 20)],
        values: vec![100],
    };

    let _rows = root.to_rows().axis_rows;
}

#[test]
#[should_panic(
    expected = "rowview index join requires axis and joined collection lengths to match"
)]
fn index_panics_when_joined_len_is_longer() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<u16>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        #[joins(index = root.values[..], as = vals)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[select(select = vals.1)]
            joined_value: Option<u16>,
        }
    }

    let root = Root {
        axis: vec![(1, 10)],
        values: vec![100, 200],
    };

    let _rows = root.to_rows().axis_rows;
}
