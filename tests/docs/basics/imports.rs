//! Macro allows imports in the schema module.

mod fixture {
    #[derive(Clone, Debug, Copy, PartialEq)]
    pub struct Imported(pub u32);
}

#[test]
fn schema_module_can_use_imports() {
    use fixture::Imported;

    struct Root {
        axis: Vec<(Imported,)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        use super::fixture::Imported;

        #[rowset(name = axis_rows, axis = root.axis)]
        struct AxisRow {
            #[from_axis(axis.0)]
            value: Imported,
        }
    }

    let root = Root {
        axis: vec![(Imported(4),), (Imported(5),), (Imported(6),)],
    };
    let rows = root.to_rows();

    assert_eq!(rows.axis_rows.len(), 3);
    assert_eq!(rows.axis_rows[0].value, Imported(4));
    assert_eq!(rows.axis_rows[1].value, Imported(5));
    assert_eq!(rows.axis_rows[2].value, Imported(6));
}
