//! Root can be provided as any reference or lifetime.

static A_VALUES: [&u32; 2] = [&1, &2];
static B_VALUES: [&u32; 2] = [&3, &4];
static C_VALUES: [&u32; 2] = [&5, &6];
static A_PAIR: [&u32; 2] = [&7, &8];
static B_PAIR: [&u32; 2] = [&9, &10];

type StaticValues = &'static [&'static u32];
type StaticValuePair = [&'static u32; 2];

#[test]
fn nested_static_reference_slices_survive_nested_axis() {
    struct Group {
        name: &'static str,
        items: Vec<Item>,
    }

    struct Item {
        key: &'static str,
        values: StaticValues,
    }

    struct Root {
        groups: Vec<Group>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        use super::StaticValues;

        #[rowset(name = item_rows, axis = root.groups[..].items)]
        struct ItemRow {
            #[copy(root.groups[..].name)]
            group_name: &'static str,
            #[from_axis(axis.key)]
            key: &'static str,
            #[from_axis(axis.values)]
            values: StaticValues,
            #[from_axis(axis.values.first().copied().unwrap())]
            first_value: &'static u32,
        }
    }

    let rows = Root {
        groups: vec![
            Group {
                name: "letters",
                items: vec![
                    Item {
                        key: "a",
                        values: &A_VALUES,
                    },
                    Item {
                        key: "b",
                        values: &B_VALUES,
                    },
                ],
            },
            Group {
                name: "numbers",
                items: vec![Item {
                    key: "one",
                    values: &C_VALUES,
                }],
            },
        ],
    }
    .to_rows()
    .item_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].group_name, "letters");
    assert_eq!(rows[0].key, "a");
    assert_eq!(rows[0].values, &A_VALUES);
    assert_eq!(rows[0].first_value, &1);
    assert_eq!(rows[1].group_name, "letters");
    assert_eq!(rows[1].first_value, &3);
    assert_eq!(rows[2].group_name, "numbers");
    assert_eq!(rows[2].first_value, &5);
}

#[test]
fn fixed_size_arrays_of_static_references_survive_axis_rows() {
    struct Root {
        pairs: Vec<(&'static str, StaticValuePair)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        use super::StaticValuePair;

        #[rowset(name = pair_rows, axis = root.pairs)]
        struct PairRow {
            #[from_axis(axis.0)]
            key: &'static str,
            #[from_axis(axis.1)]
            pair: StaticValuePair,
            #[from_axis(axis.1.get(1).copied().unwrap())]
            second: &'static u32,
        }
    }

    let rows = Root {
        pairs: vec![("a", A_PAIR), ("b", B_PAIR)],
    }
    .to_rows()
    .pair_rows;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].key, "a");
    assert_eq!(rows[0].pair, A_PAIR);
    assert_eq!(rows[0].second, &8);
    assert_eq!(rows[1].key, "b");
    assert_eq!(rows[1].pair, B_PAIR);
    assert_eq!(rows[1].second, &10);
}

#[test]
fn borrowed_lookup_slice_joins_to_reference_slice_axis() {
    struct Root {
        groups: Vec<(&'static str, StaticValues)>,
        overrides: &'static [(&'static str, &'static u32)],
    }

    #[rowview::rows(root = Root)]
    mod schema {
        use super::StaticValues;

        #[rowset(name = group_rows, axis = root.groups)]
        #[joins(left = root.overrides, as = override_value, on(axis.0 = override_value.0))]
        struct GroupRow {
            #[from_axis(axis.0)]
            key: &'static str,
            #[from_axis(axis.1)]
            values: StaticValues,
            #[from_axis(axis.1.first().copied().unwrap())]
            first: &'static u32,
            #[select(select = override_value.1)]
            override_value: Option<&'static u32>,
        }
    }

    let rows = Root {
        groups: vec![("a", &A_VALUES), ("b", &B_VALUES), ("c", &C_VALUES)],
        overrides: &[("b", &10)],
    }
    .to_rows()
    .group_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].key, "a");
    assert_eq!(rows[0].values, &A_VALUES);
    assert_eq!(rows[0].first, &1);
    assert_eq!(rows[0].override_value, None);
    assert_eq!(rows[1].key, "b");
    assert_eq!(rows[1].first, &3);
    assert_eq!(rows[1].override_value, Some(&10));
    assert_eq!(rows[2].key, "c");
    assert_eq!(rows[2].first, &5);
    assert_eq!(rows[2].override_value, None);
}

#[test]
fn sum_nested_reference_values_into_axis_rows() {
    struct Root {
        a: Vec<(u64, B)>,
    }

    struct B {
        cs: Vec<&'static u32>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = abcs, axis = root.a)]
        struct Abcs {
            #[agg(sum = axis.1.cs)]
            cs: u32,
        }
    }

    let rows = Root {
        a: vec![
            (1, B { cs: vec![&10, &20] }),
            (
                2,
                B {
                    cs: vec![&3, &4, &5],
                },
            ),
        ],
    }
    .to_rows()
    .abcs;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].cs, 30);
    assert_eq!(rows[1].cs, 12);
}
