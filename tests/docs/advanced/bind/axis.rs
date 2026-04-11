//! Forms virtual axis from several collection by join.
//! Virtual axis can join.
//! Multiple optional sources can project into one rowset.
//! If any axis exists, try to join with another axis.
// Multi-hop joins need dependency semantics:
// Given axes x, y, and a:
// - if x and y exist, y must exist for x
// - if y exists, a must exist for y
// - otherwise the whole chain is optional
// #[join_chain(option, x -> y must -> a must, select = a.value)]

#[test]
fn a_rows_join_multiple_optional_sources() {
    struct B {
        a: i64,
        b: i64,
        c: i64,
    }

    struct Root {
        a: Vec<u32>,
        b: Vec<(u32, i64)>,
        c: Vec<(u32, i64)>,
        d: Vec<(u32, i64)>,
        e: Vec<(u32, i64)>,
        f: Vec<(u32, B)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = a_rows, axis = root.a)]
        #[joins(left = root.b[..], as = b, on = *axis == b.0)]
        #[joins(left = root.c[..], as = c, on = *axis == c.0)]
        #[joins(left = root.d[..], as = d, on = *axis == d.0)]
        #[joins(left = root.e[..], as = e, on = *axis == e.0)]
        #[joins(left = root.f[..], as = f, on = *axis == f.0)]
        struct A {
            #[from_axis(*axis)]
            a: u32,
            #[select(select = b.1)]
            b: Option<i64>,
            #[select(select = c.1)]
            c: Option<i64>,
            #[select(select = d.1)]
            d: Option<i64>,
            #[select(select = e.1)]
            e: Option<i64>,
            #[select(select = f.1.a)]
            f_a: Option<i64>,
            #[select(select = f.1.b)]
            f_b: Option<i64>,
            #[select(select = f.1.c)]
            f_c: Option<i64>,
        }
    }

    let rows = Root {
        a: vec![1, 2, 3],
        b: vec![(1, 100), (3, -30)],
        c: vec![(1, 7), (2, 11)],
        d: vec![(2, 2000), (3, -3000)],
        e: vec![(1, 70), (3, 90)],
        f: vec![
            (
                1,
                B {
                    a: 10,
                    b: 700,
                    c: 1000,
                },
            ),
            (
                3,
                B {
                    a: -30,
                    b: 900,
                    c: 3000,
                },
            ),
        ],
    }
    .to_rows()
    .a_rows;

    assert_eq!(rows.len(), 3);

    assert_eq!(rows[0].a, 1);
    assert_eq!(rows[0].b, Some(100));
    assert_eq!(rows[0].c, Some(7));
    assert_eq!(rows[0].d, None);
    assert_eq!(rows[0].e, Some(70));
    assert_eq!(rows[0].f_a, Some(10));
    assert_eq!(rows[0].f_b, Some(700));
    assert_eq!(rows[0].f_c, Some(1000));

    assert_eq!(rows[1].a, 2);
    assert_eq!(rows[1].b, None);
    assert_eq!(rows[1].c, Some(11));
    assert_eq!(rows[1].d, Some(2000));
    assert_eq!(rows[1].e, None);
    assert_eq!(rows[1].f_a, None);
    assert_eq!(rows[1].f_b, None);
    assert_eq!(rows[1].f_c, None);

    assert_eq!(rows[2].a, 3);
    assert_eq!(rows[2].b, Some(-30));
    assert_eq!(rows[2].c, None);
    assert_eq!(rows[2].d, Some(-3000));
    assert_eq!(rows[2].e, Some(90));
    assert_eq!(rows[2].f_a, Some(-30));
    assert_eq!(rows[2].f_b, Some(900));
    assert_eq!(rows[2].f_c, Some(3000));
}
