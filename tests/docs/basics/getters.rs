//! Usage of getters - map of original struct can be done via .get() method.

use std::collections::BTreeMap;

#[test]
fn map_getter_on_root_works_with_axis_argument() {
    struct Root {
        axis: Vec<(u32,)>,
        names: BTreeMap<u32, &'static str>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[copy(root.names.get(&axis.0).copied().unwrap())]
            name: &'static str,
        }
    }

    let root = Root {
        axis: vec![(1,), (3,), (2,)],
        names: BTreeMap::from([(1, "alpha"), (2, "beta"), (3, "gamma")]),
    };

    let rows = root.to_rows().axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].name, "alpha");
    assert_eq!(rows[1].id, 3);
    assert_eq!(rows[1].name, "gamma");
    assert_eq!(rows[2].id, 2);
    assert_eq!(rows[2].name, "beta");
}
