//! Multiple row-level joins.

#[test]
fn support_union_creates_rows_and_bindings_project_optionally() {
    struct A {
        id: u32,
        value: &'static str,
    }

    struct B {
        id: u32,
        value: &'static str,
    }

    struct C {
        id: &'static u32,
        value: &'static str,
    }

    struct D {
        id: u32,
        value: &'static str,
    }

    struct Root {
        a: Vec<A>,
        b: Vec<B>,
        c: Vec<C>,
        d: Vec<D>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = rows)]
        #[support(any(root.a[..].id, root.b[..].id))]
        #[bind(left = root.a, as = a, by = a.id)]
        #[bind(left = root.b, as = b, by = b.id)]
        #[bind(left = root.d, as = d, by = d.id)]
        #[bind(
            left = root.c,
            as = c,
            by = *c.id,
            on = all(any(a, b), not(d))
        )]
        struct Row {
            #[from_key(key)]
            id: u32,
            #[select(select = a.value)]
            a_value: Option<&'static str>,
            #[select(select = b.value)]
            b_value: Option<&'static str>,
            #[select(select = c.value)]
            c_value: Option<&'static str>,
            #[select(select = d.value)]
            d_value: Option<&'static str>,
        }
    }

    let rows = Root {
        a: vec![A { id: 1, value: "a1" }],
        b: vec![B { id: 2, value: "b2" }],
        c: vec![
            C {
                id: &1,
                value: "c1",
            },
            C {
                id: &2,
                value: "c2",
            },
            C {
                id: &3,
                value: "c3",
            },
        ],
        d: vec![D { id: 2, value: "d2" }],
    }
    .to_rows()
    .rows;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].a_value, Some("a1"));
    assert_eq!(rows[0].b_value, None);
    assert_eq!(rows[0].c_value, Some("c1"));
    assert_eq!(rows[0].d_value, None);
    assert_eq!(rows[1].id, 2);
    assert_eq!(rows[1].a_value, None);
    assert_eq!(rows[1].b_value, Some("b2"));
    assert_eq!(rows[1].c_value, None);
    assert_eq!(rows[1].d_value, Some("d2"));
}

#[test]
fn inner_join_skips_axis_rows_without_match() {
    struct Root {
        ids: Vec<u32>,
        values: Vec<(u32, &'static str)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = rows, axis = root.ids)]
        #[joins(inner = root.values[..], as = value, on = *axis == value.0)]
        struct Row {
            #[from_axis(*axis)]
            id: u32,
            #[select(select = value.1)]
            value: &'static str,
        }
    }

    let rows = Root {
        ids: vec![1, 2, 3],
        values: vec![(1, "one"), (3, "three")],
    }
    .to_rows()
    .rows;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].value, "one");
    assert_eq!(rows[1].id, 3);
    assert_eq!(rows[1].value, "three");
}
