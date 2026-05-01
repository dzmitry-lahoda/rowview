//! Tests which ensure generated code does not clone input data.
#![forbid(clippy::clone_on_copy, clippy::redundant_clone)]

struct CloneTrap {
    bytes: Vec<u8>,
}

impl Clone for CloneTrap {
    fn clone(&self) -> Self {
        panic!("CloneTrap should not be cloned");
    }
}

impl CloneTrap {
    fn first(&self) -> u8 {
        self.bytes[0]
    }
}

struct Root {
    a: (u32, CloneTrap),
    b: Vec<(u32, CloneTrap)>,
}

#[rowview::rows(root = Root)]
mod schema {
    #[rowset(name = a_rows, axis = ())]
    struct A {
        #[copy(root.a.0)]
        a: u32,
        #[copy(root.a.1.first())]
        b: u8,
    }

    #[rowset(name = b_rows, axis = root.b)]
    struct B {
        #[copy(root.a.0)]
        a: u32,
        #[from_axis(axis.0)]
        b: u32,
        #[from_axis(axis.1.first())]
        c: u8,
    }
}

#[test]
fn generated_rows_do_not_clone_input_data() {
    let root = Root {
        a: (
            7,
            CloneTrap {
                bytes: vec![1, 2, 3],
            },
        ),
        b: vec![
            (10, CloneTrap { bytes: vec![4] }),
            (11, CloneTrap { bytes: vec![5, 6] }),
        ],
    };

    let rows = root.to_rows();

    assert_eq!(rows.a_rows.len(), 1);
    assert_eq!(rows.a_rows.a[0], 7);
    assert_eq!(rows.a_rows.b[0], 1);

    assert_eq!(rows.b_rows.len(), 2);
    assert_eq!(rows.b_rows.a[0], 7);
    assert_eq!(rows.b_rows.b[0], 10);
    assert_eq!(rows.b_rows.c[0], 4);
    assert_eq!(rows.b_rows.a[1], 7);
    assert_eq!(rows.b_rows.b[1], 11);
    assert_eq!(rows.b_rows.c[1], 5);
}
