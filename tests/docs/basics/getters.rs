//! Usage of getters - map of original struct can be done via .get() method.

use std::collections::BTreeMap;

#[test]
fn map_getter_on_axis_works() {
    struct Root {
        axis: Vec<(u32, BTreeMap<u32, u32>)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[from_axis(axis.1.get(&axis.0).copied().unwrap())]
            value: u32,
        }
    }

    let root = Root {
        axis: vec![
            (1, BTreeMap::from([(1, 10), (2, 20)])),
            (3, BTreeMap::from([(3, 30), (4, 40)])),
            (2, BTreeMap::from([(1, 50), (2, 60)])),
        ],
    };

    let rows = root.to_rows().axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].value, 10);
    assert_eq!(rows[1].id, 3);
    assert_eq!(rows[1].value, 30);
    assert_eq!(rows[2].id, 2);
    assert_eq!(rows[2].value, 60);
}

#[test]
fn map_getter_on_root_works_with_copy() {
    struct Root {
        axis: Vec<(u32,)>,
        names: BTreeMap<u32, u32>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[copy(root.names.get(&axis.0).copied().unwrap())]
            value: u32,
        }
    }

    let root = Root {
        axis: vec![(1,), (3,), (2,)],
        names: BTreeMap::from([(1, 10), (2, 20), (3, 30)]),
    };

    let rows = root.to_rows().axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].value, 10);
    assert_eq!(rows[1].id, 3);
    assert_eq!(rows[1].value, 30);
    assert_eq!(rows[2].id, 2);
    assert_eq!(rows[2].value, 20);
}
