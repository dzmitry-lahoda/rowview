use rowview::query::*;

#[test]
fn vec_tuple_vec_tuple_into_option() {
    struct Root {
        axis: Vec<(u32, u8)>,
        values: Vec<(u32, u16)>,
    }

    #[rowview::select]
    struct AxisRow {
        id: u32,
        direct_value: u8,
        joined_value_key: Option<u32>,
        joined_value: Option<u16>,
    }

    let root = Root {
        axis: vec![(1, 10), (2, 20), (3, 30)],
        values: vec![(2, 200), (3, 300), (4, 400)],
    };

    let rows = select::<AxisRow>::from(&root.axis)
        .join_left(&root.values, on(axis::_0.eq(vals::_0)))
        .project(
            (axis::_0, axis::_1, vals::_0.some(), vals::_1.some()),
            (axis::_0, axis::_1, none::<u32>(), none::<u16>()),
        )
        .execute();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows.id[0], 1);
    assert_eq!(rows.direct_value[0], 10);
    assert_eq!(rows.joined_value_key[0], None);
    assert_eq!(rows.joined_value[0], None);
    assert_eq!(rows.id[1], 2);
    assert_eq!(rows.direct_value[1], 20);
    assert_eq!(rows.joined_value_key[1], Some(2));
    assert_eq!(rows.joined_value[1], Some(200));
    assert_eq!(rows.id[2], 3);
    assert_eq!(rows.direct_value[2], 30);
    assert_eq!(rows.joined_value_key[2], Some(3));
    assert_eq!(rows.joined_value[2], Some(300));
}
