//! Usage of getters: mapping from the original struct can be done via the .get() method.

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
    assert_eq!(rows.id[0], 1);
    assert_eq!(rows.value[0], 10);
    assert_eq!(rows.id[1], 3);
    assert_eq!(rows.value[1], 30);
    assert_eq!(rows.id[2], 2);
    assert_eq!(rows.value[2], 60);
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
    assert_eq!(rows.id[0], 1);
    assert_eq!(rows.value[0], 10);
    assert_eq!(rows.id[1], 3);
    assert_eq!(rows.value[1], 30);
    assert_eq!(rows.id[2], 2);
    assert_eq!(rows.value[2], 20);
}

#[test]
fn immutable_root_getter_method_on_axis_forms_column() {
    struct Root {
        ids: Vec<u32>,
    }

    impl Root {
        fn get(&self) -> Vec<u32> {
            self.ids.clone()
        }
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.get())]
        struct AxisRow {
            #[from_axis(*axis)]
            id: u32,
        }
    }

    let root = Root { ids: vec![1, 3, 2] };

    let rows = root.to_rows().axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows.id[0], 1);
    assert_eq!(rows.id[1], 3);
    assert_eq!(rows.id[2], 2);
}
