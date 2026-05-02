//! Full join, panics if not found.

use rowview::query::*;

#[test]
fn vec_tuple_vec_tuple_into_value_2() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::select]
    struct AxisRow {
        id: u32,
        direct_value: u8,
        joined_value_key: u32,
        joined_value: u16,
    }

    let root = Root {
        axis: vec![(1, 10), (2, 20), (3, 30)],
        values: vec![(1, 100), (2, 200), (3, 300)],
    };

    let rows = select::<AxisRow>::from(&root.axis)
        .join_must(&root.values, on(axis::_0.eq(vals::_0)))
        .project((axis::_0, axis::_1, vals::_0, vals::_1))
        .execute();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows.id[0], 1);
    assert_eq!(rows.direct_value[0], 10);
    assert_eq!(rows.joined_value_key[0], 1);
    assert_eq!(rows.joined_value[0], 100);
    assert_eq!(rows.id[1], 2);
    assert_eq!(rows.direct_value[1], 20);
    assert_eq!(rows.joined_value_key[1], 2);
    assert_eq!(rows.joined_value[1], 200);
    assert_eq!(rows.id[2], 3);
    assert_eq!(rows.direct_value[2], 30);
    assert_eq!(rows.joined_value_key[2], 3);
    assert_eq!(rows.joined_value[2], 300);
}

#[test]
fn panics_when_item_not_found() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::select]
    struct AxisRow {
        id: u32,
        joined_value: u16,
    }

    let panic = scoped_panic_hook::catch_panic(|| {
        let root = Root {
            axis: vec![(1, 10), (2, 20)],
            values: vec![(1, 100)],
        };

        let _rows = select::<AxisRow>::from(&root.axis)
            .join_must(&root.values, on(axis::_0.eq(vals::_0)))
            .project((axis::_0, vals::_1))
            .execute();
    })
    .expect_err(crate::docs::MISSING_MUST_JOIN_SHOULD_PANIC);

    assert!(
        panic
            .display_with_backtrace()
            .to_string()
            .contains("rowview must join found no matching item")
    );
}
